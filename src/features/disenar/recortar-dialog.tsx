import { Loader2 } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { ArtRef } from "@/bindings/ArtRef";
import type { DeckCard } from "@/bindings/DeckCard";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
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

type Rect = { x: number; y: number; w: number; h: number };

/** La zona de la ilustración en una carta normal (más o menos): para empezar. */
const ZONA_INICIAL: Rect = { x: 0.08, y: 0.11, w: 0.84, h: 0.45 };

/**
 * «Recortar de una carta ya hecha»: se elige una carta del deck (sin diseño,
 * con su imagen) y se marca la zona de la ilustración arrastrando.
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
	const inicio = useRef<{ x: number; y: number } | null>(null);

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
		(e.target as HTMLElement).setPointerCapture(e.pointerId);
		inicio.current = punto(e);
		setZona({ ...inicio.current, w: 0, h: 0 });
	};
	const alMover = (e: React.PointerEvent) => {
		const a = inicio.current;
		if (!a) return;
		const p = punto(e);
		setZona({
			x: Math.min(a.x, p.x),
			y: Math.min(a.y, p.y),
			w: Math.abs(p.x - a.x),
			h: Math.abs(p.y - a.y),
		});
	};
	const alSoltar = () => {
		inicio.current = null;
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
				<DialogContent className="flex max-h-[92vh] flex-col sm:max-w-3xl">
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
								className="relative aspect-[63/88] h-[min(68vh,720px)] touch-none select-none"
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
								/>
							</div>
						</div>
					) : (
						<ScrollArea className="min-h-0 flex-1">
							<div className="grid grid-cols-5 gap-2 p-1 pr-2.5">
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
									<p className="col-span-5 text-xs text-muted-foreground">
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
							{ocupado && <Loader2 className="animate-spin" />}
							{t("Usar esta zona")}
						</Button>
					</DialogFooter>
				</DialogContent>
			)}
		</Dialog>
	);
}
