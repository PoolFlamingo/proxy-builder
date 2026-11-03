import { useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { ArtRef } from "@/bindings/ArtRef";
import type { DeckCard } from "@/bindings/DeckCard";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { EDICION_NEUTRA, parametros, urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { type Agarre, agarreEn, arrastrar, type Rect } from "./rectangulo";

/** Hasta dónde se agarra una esquina de la zona con el dedo, en px. */
const RADIO_DEDO = 30;

/** Dónde va cada tirador: centrado en su esquina de la zona. */
const ESQUINAS = [
	"top-0 left-0 -translate-x-1/2 -translate-y-1/2",
	"top-0 right-0 translate-x-1/2 -translate-y-1/2",
	"bottom-0 left-0 -translate-x-1/2 translate-y-1/2",
	"right-0 bottom-0 translate-x-1/2 translate-y-1/2",
];

/** La zona de la ilustración en una carta normal (más o menos): para empezar. */
const ZONA_INICIAL: Rect = { x: 0.08, y: 0.11, w: 0.84, h: 0.45 };

/**
 * «Recortar de una carta ya hecha»: se elige una carta del deck (sin diseño,
 * con su imagen) y se marca la zona de la ilustración arrastrando. Con el
 * ratón, cada arrastre marca otra. Con el dedo se agarra una esquina para
 * cambiarla de tamaño, el interior para moverla, o se arrastra por fuera para
 * marcar otra (`rectangulo.ts`).
 */
export function RecortarDialog({
	abierto,
	onCerrar,
	onRecortado,
}: {
	abierto: boolean;
	onCerrar: () => void;
	onRecortado: (a: ArtRef) => void;
}) {
	const { t, mensaje } = useLang();
	const cartas = useDeck((s) => s.deck?.cards);
	const ajustes = useDeck((s) => s.deck?.settings);
	const [elegida, setElegida] = useState<DeckCard | null>(null);
	const [zona, setZona] = useState<Rect>(ZONA_INICIAL);
	const [ocupado, setOcupado] = useState(false);
	const marco = useRef<HTMLDivElement>(null);
	// Lo que se agarró al tocar: qué, desde dónde y cómo estaba la zona.
	const toque = useRef<{
		agarre: Agarre;
		desde: { x: number; y: number };
		base: Rect;
		id: number;
	} | null>(null);

	// Una por imagen: sin repetir copias ni cartas sin imagen.
	const conImagen = useMemo(() => {
		const vistas = new Set<string>();
		return (cartas ?? []).filter((c) => {
			if (!c.front || c.front.origin === "designed" || vistas.has(c.front.blob)) return false;
			vistas.add(c.front.blob);
			return true;
		});
	}, [cartas]);

	const punto = (e: React.PointerEvent) => {
		const r = marco.current?.getBoundingClientRect();
		if (!r) return { x: 0, y: 0 };
		return {
			x: Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)),
			y: Math.min(1, Math.max(0, (e.clientY - r.top) / r.height)),
		};
	};
	const alBajar = (e: React.PointerEvent) => {
		const raton = e.pointerType === "mouse";
		// Un solo dedo a la vez: el segundo no cambia lo que ya se agarró.
		if (toque.current && !raton) return;
		const r = marco.current?.getBoundingClientRect();
		if (!r) return;
		e.currentTarget.setPointerCapture(e.pointerId);
		const desde = punto(e);
		// Con el ratón, como siempre: pulsar en cualquier sitio marca otra zona. Las
		// asas y mover por dentro son del dedo (marcarla de nuevo es difícil a ojo).
		const agarre = raton ? "nuevo" : agarreEn(zona, desde, { w: r.width, h: r.height }, RADIO_DEDO);
		toque.current = { agarre, desde, base: zona, id: e.pointerId };
		// Marcar otra desde cero: empieza vacía en el punto tocado.
		if (agarre === "nuevo") setZona({ ...desde, w: 0, h: 0 });
	};
	const alMover = (e: React.PointerEvent) => {
		const a = toque.current;
		if (!a || a.id !== e.pointerId) return;
		setZona(arrastrar(a.agarre, a.base, a.desde, punto(e)));
	};
	const alSoltar = (e: React.PointerEvent) => {
		if (toque.current?.id === e.pointerId) toque.current = null;
	};

	const recortar = async () => {
		if (!elegida?.front) return;
		setOcupado(true);
		try {
			onRecortado(await call("art_from_card", { blob: elegida.front.blob, ...zona }));
			setElegida(null);
			setZona(ZONA_INICIAL);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(false);
		}
	};

	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && onCerrar()}>
			{abierto && (
				<DialogoAdaptable className="flex max-h-[92vh] flex-col sm:max-w-3xl">
					<DialogHeader>
						<DialogTitle>{t("Recortar la ilustración de una carta")}</DialogTitle>
						<DialogDescription className="text-xs">
							{elegida
								? t("Arrastra sobre la carta para marcar la zona de la ilustración.")
								: t("Elige la carta ya hecha de la que sacar la ilustración.")}
						</DialogDescription>
					</DialogHeader>
					{elegida?.front && ajustes ? (
						<div className="flex min-h-0 flex-1 items-center justify-center">
							<div
								ref={marco}
								className="relative aspect-[63/88] w-[min(100%,calc(min(68dvh,720px)*63/88))] touch-none select-none"
								onPointerDown={alBajar}
								onPointerMove={alMover}
								onPointerUp={alSoltar}
								onPointerCancel={alSoltar}
							>
								<img
									src={urlMiniatura(
										elegida.front.blob,
										parametros(ajustes, EDICION_NEUTRA, false, 0),
										745,
									)}
									alt=""
									draggable={false}
									className="size-full rounded-[4.5%] object-cover"
								/>
								<div
									className="pointer-events-none absolute border-2 border-cyan-400 bg-cyan-400/10 shadow-[0_0_0_9999px_rgba(0,0,0,0.45)]"
									style={{
										left: `${zona.x * 100}%`,
										top: `${zona.y * 100}%`,
										width: `${zona.w * 100}%`,
										height: `${zona.h * 100}%`,
									}}
								>
									{/* Las esquinas que se agarran con el dedo para cambiar el tamaño. */}
									{ESQUINAS.map((pos) => (
										<span
											key={pos}
											className={cn(
												"absolute hidden size-5 rounded-[2px] border-2 border-cyan-400 bg-background [[data-tactil]_&]:block",
												pos,
											)}
										/>
									))}
								</div>
							</div>
						</div>
					) : (
						<ScrollArea className="min-h-0 flex-1">
							<div className="grid grid-cols-3 gap-2 p-1 pr-2.5 sm:grid-cols-5">
								{conImagen.map((c) =>
									c.front && ajustes ? (
										<button
											key={c.id}
											type="button"
											onClick={() => setElegida(c)}
											className="overflow-hidden rounded-[4.5%] ring-primary hover:ring-2"
										>
											<img
												src={urlMiniatura(
													c.front.blob,
													parametros(ajustes, c.edits, false, 0),
													244,
												)}
												alt={c.meta.name}
												loading="lazy"
												className="aspect-[63/88] w-full object-cover"
											/>
										</button>
									) : null,
								)}
								{conImagen.length === 0 && (
									<p className="col-span-3 text-xs text-muted-foreground sm:col-span-5">
										{t("No hay cartas con imagen en el deck.")}
									</p>
								)}
							</div>
						</ScrollArea>
					)}
					<DialogFooter className="sm:justify-between">
						<Button
							size="sm"
							variant="ghost"
							className={cn(!elegida && "invisible")}
							onClick={() => setElegida(null)}
						>
							{t("Elegir otra carta")}
						</Button>
						<Button
							size="sm"
							onClick={recortar}
							disabled={!elegida || ocupado || zona.w < 0.05 || zona.h < 0.05}
						>
							{ocupado && <Icono icon="lucide:loader-2" className="animate-spin" />}
							{t("Usar esta zona")}
						</Button>
					</DialogFooter>
				</DialogoAdaptable>
			)}
		</Dialog>
	);
}
