import { useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { IdCarta } from "@/bindings/IdCarta";
import type { Nota } from "@/bindings/Nota";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	ContextMenu,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
	Dialog,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { NINGUNA, useMesa } from "@/stores/mesa-store";
import { useRepeticion } from "@/stores/repeticion-store";
import { hacer } from "./acciones";
import { MAX_NOTA } from "./cuentas-notas";
import { usePantalla } from "./pantalla";

const cerrar = () => usePantalla.getState().abrir(null);

/** Cambia una nota propia sin tocar lo demás. */
function editar(n: Nota, cambios: Partial<Pick<Nota, "texto" | "temporal" | "privada">>) {
	return hacer({
		que: "editarNota",
		id: n.id,
		texto: n.texto,
		temporal: n.temporal,
		privada: n.privada,
		...cambios,
	});
}

/** Empieza a elegir la carta del campo a la que se ata esta nota (el siguiente clic). */
export function atarNota(nota: number) {
	usePantalla.getState().setModo({ tipo: "nota", nota });
}

/** Suelta de su carta todas mis notas atadas a ella. */
export function soltarNotasDe(carta: IdCarta) {
	const m = useMesa.getState();
	for (const n of m.general?.notas ?? []) {
		if (n.carta === carta && n.autor === m.yo)
			void hacer({ que: "atarNota", id: n.id, carta: null });
	}
}

/**
 * **Escribir o cambiar una nota** desde la mesa: suelta (clic derecho en el
 * campo vacío) o ya atada a una carta (clic derecho en la carta). Con el texto
 * van si es temporal (se borra al pasar el turno) y si es privada.
 */
export function NotaDialog({ carta, nota }: { carta: IdCarta | null; nota?: Nota }) {
	const { t } = useLang();
	const nombre = useMesa((s) => (carta !== null ? s.cartas[carta]?.origen?.nombre : undefined));
	const [texto, setTexto] = useState(nota?.texto ?? "");
	const [temporal, setTemporal] = useState(nota?.temporal ?? false);
	const [privada, setPrivada] = useState(nota?.privada ?? false);
	const limpio = texto.trim();
	const hecho = () => {
		if (!limpio) return;
		cerrar();
		if (nota) void editar(nota, { texto: limpio, temporal, privada });
		else void hacer({ que: "notaEnCarta", texto: limpio, temporal, privada, carta });
	};
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-sm" cambiosSinGuardar={limpio !== (nota?.texto ?? "")}>
				<DialogHeader>
					<DialogTitle>
						{nota
							? t("Editar nota")
							: nombre
								? t("Nota en {carta}", { carta: nombre })
								: t("Nueva nota")}
					</DialogTitle>
				</DialogHeader>
				<Input
					autoFocus
					value={texto}
					maxLength={MAX_NOTA}
					placeholder={t("Apunta algo: el maná de un tesoro, un recordatorio…")}
					onChange={(e) => setTexto(e.target.value)}
					onKeyDown={(e) => {
						if (e.key === "Enter") {
							e.preventDefault();
							hecho();
						}
					}}
				/>
				<div className="flex flex-wrap gap-1">
					<Button
						size="xs"
						variant={temporal ? "secondary" : "outline"}
						aria-pressed={temporal}
						onClick={() => setTemporal((x) => !x)}
					>
						<Icono icon="game-icons:hourglass" className="size-3" />
						{temporal ? t("Temporal") : t("Fija")}
					</Button>
					<Button
						size="xs"
						variant={privada ? "secondary" : "outline"}
						aria-pressed={privada}
						onClick={() => setPrivada((x) => !x)}
					>
						<Icono icon={privada ? "lucide:lock" : "lucide:lock-open"} className="size-3" />
						{privada ? t("Privada") : t("Pública")}
					</Button>
				</div>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={cerrar}>
						{t("Cancelar")}
					</Button>
					<Button size="sm" onClick={hecho} disabled={!limpio}>
						{nota ? t("Guardar") : t("Añadir nota")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

/**
 * **Las notas sueltas sobre la mesa**, arriba a la izquierda del campo, como
 * pósits del color de su autor. Las mías se atan a una carta con un clic (y
 * luego el clic en la carta) y tienen su menú; las atadas a una carta se ven
 * en la carta (`insignias.tsx`). Escape cancela el modo de atar (`atajos.ts`).
 */
export function NotasSueltas() {
	const { t } = useLang();
	const notas = useMesa(
		useShallow((s) => (s.general?.notas ?? NINGUNA).filter((n) => n.carta == null)),
	);
	const yo = useMesa((s) => s.yo);
	const jugadores = useMesa((s) => s.jugadores);
	const repitiendo = useRepeticion((s) => s.activa !== null);
	const atando = usePantalla((s) => (s.modo?.tipo === "nota" ? s.modo.nota : null));
	if (notas.length === 0) return null;
	return (
		<div
			className="absolute top-14 left-3 flex max-h-[55%] w-44 flex-col gap-1.5 overflow-hidden"
			style={{ zIndex: 5500 }}
		>
			{notas.map((n) => {
				const mia = n.autor === yo && !repitiendo;
				const color = jugadores[n.autor]?.color ?? "#facc15";
				const pegote = (
					<button
						type="button"
						disabled={!mia}
						onClick={() => mia && atarNota(n.id)}
						className={cn(
							"flex w-full items-start gap-1 rounded-md bg-amber-100 px-2 py-1 text-left text-[11px] leading-snug text-stone-900 shadow-md ring-1 ring-black/10 dark:bg-amber-200",
							mia && "cursor-pointer hover:ring-2 hover:ring-primary",
							atando === n.id && "ring-2 ring-primary",
						)}
						style={{ borderLeft: `4px solid ${color}` }}
					>
						<span className="line-clamp-3 min-w-0 flex-1 break-words whitespace-pre-wrap">
							{n.texto}
						</span>
						<span className="flex shrink-0 flex-col items-center gap-0.5 pt-0.5 opacity-60">
							{n.temporal && <Icono icon="game-icons:hourglass" className="size-3" />}
							{n.privada && <Icono icon="lucide:lock" className="size-3" />}
						</span>
					</button>
				);
				if (!mia)
					return (
						<WithTooltip key={n.id} content={jugadores[n.autor]?.nombre ?? ""}>
							{pegote}
						</WithTooltip>
					);
				return (
					<ContextMenu key={n.id}>
						<WithTooltip content={t("Pulsa y elige una carta para atarla")}>
							<ContextMenuTrigger asChild>{pegote}</ContextMenuTrigger>
						</WithTooltip>
						<MenuNota nota={n} />
					</ContextMenu>
				);
			})}
		</div>
	);
}

/** El menú de una nota mía: editarla, atarla, temporal o fija, privada o pública, borrarla. */
function MenuNota({ nota }: { nota: Nota }) {
	const { t } = useLang();
	return (
		<ContenidoContextual className="w-56 text-xs">
			<ContextMenuItem
				onSelect={() =>
					usePantalla.getState().abrir({ tipo: "nota", carta: nota.carta ?? null, nota })
				}
			>
				<Icono icon="lucide:pencil" />
				{t("Editar nota…")}
			</ContextMenuItem>
			{nota.carta == null ? (
				<ContextMenuItem onSelect={() => atarNota(nota.id)}>
					<Icono icon="game-icons:pin" />
					{t("Atar a una carta")}
				</ContextMenuItem>
			) : (
				<ContextMenuItem onSelect={() => void hacer({ que: "atarNota", id: nota.id, carta: null })}>
					<Icono icon="lucide:unlink" />
					{t("Soltar de la carta")}
				</ContextMenuItem>
			)}
			<ContextMenuSeparator />
			<ContextMenuItem onSelect={() => void editar(nota, { temporal: !nota.temporal })}>
				<Icono icon="game-icons:hourglass" />
				{nota.temporal ? t("Hacerla fija") : t("Hacerla temporal")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => void editar(nota, { privada: !nota.privada })}>
				<Icono icon={nota.privada ? "lucide:lock-open" : "lucide:lock"} />
				{nota.privada ? t("Hacerla pública") : t("Hacerla privada")}
			</ContextMenuItem>
			<ContextMenuSeparator />
			<ContextMenuItem
				variant="destructive"
				onSelect={() => void hacer({ que: "quitarNota", id: nota.id })}
			>
				<Icono icon="lucide:trash-2" />
				{t("Borrar nota")}
			</ContextMenuItem>
		</ContenidoContextual>
	);
}

/**
 * Lo de las notas en el menú de una carta del campo: escribir una atada a
 * ella y, de las mías que lleva, editarlas o soltarlas.
 */
export function NotasEnMenuCarta({ carta }: { carta: IdCarta }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo);
	const mias = useMesa(
		useShallow((s) =>
			(s.general?.notas ?? NINGUNA).filter((n) => n.carta === carta && n.autor === s.yo),
		),
	);
	const repitiendo = useRepeticion((s) => s.activa !== null);
	if (yo === null || repitiendo) return null;
	return (
		<>
			<ContextMenuItem onSelect={() => usePantalla.getState().abrir({ tipo: "nota", carta })}>
				<Icono icon="lucide:sticky-note" />
				{t("Añadir nota…")}
			</ContextMenuItem>
			{mias.map((n) => (
				<ContextMenuItem
					key={n.id}
					onSelect={() => usePantalla.getState().abrir({ tipo: "nota", carta, nota: n })}
				>
					<Icono icon="lucide:pencil" />
					<span className="truncate">{t("Editar «{texto}»", { texto: n.texto })}</span>
				</ContextMenuItem>
			))}
			{mias.length > 0 && (
				<ContextMenuItem onSelect={() => soltarNotasDe(carta)}>
					<Icono icon="lucide:unlink" />
					{t("Soltar mis notas")}
				</ContextMenuItem>
			)}
		</>
	);
}
