import { useMemo } from "react";
import { useShallow } from "zustand/react/shallow";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { hacer } from "../acciones";
import { type Comandante, comandantesDe, deltaSeguro, impuestoDe } from "../dano-comandante";
import { usePantalla } from "../pantalla";

const cerrar = () => usePantalla.getState().abrir(null);

/**
 * **La tabla de daño de comandante**: quién ha recibido cuánto de cada
 * comandante de la mesa (las filas son los jugadores y las columnas, los
 * comandantes), y bajo cada comandante las veces que se ha lanzado y su
 * impuesto. Es solo apuntar: la vida la baja el motor con cada punto.
 */
export function DanoComandanteDialog() {
	const { t } = useLang();
	const cartas = useMesa((s) => s.cartas);
	// `comandantesDe` hace objetos nuevos: se calcula aparte del selector.
	const comandantes = useMemo(() => comandantesDe(cartas), [cartas]);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const limite = useMesa((s) => s.opciones?.limiteComandante ?? 21);
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-4xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Daño de comandante")}</DialogTitle>
					<DialogDescription>
						{t(
							"Cuánto ha recibido cada jugador de cada comandante. Con {limite} o más de uno solo, queda fuera.",
							{ limite },
						)}
					</DialogDescription>
				</DialogHeader>
				{comandantes.length === 0 ? (
					<p className="p-2.5 text-center text-xs text-muted-foreground">
						{t("No hay comandantes en la mesa.")}
					</p>
				) : (
					<ScrollArea className="max-h-[60vh]">
						<table
							className="w-full table-fixed border-separate border-spacing-1 text-xs"
							style={{ minWidth: 112 + comandantes.length * 132 }}
						>
							<thead>
								<tr>
									<th className="w-28 text-left font-medium text-muted-foreground">
										{t("Jugador")}
									</th>
									{comandantes.map((c) => (
										<EncabezadoComandante key={c.id} c={c} />
									))}
								</tr>
							</thead>
							<tbody>
								{asientos.map((j) => (
									<FilaJugador key={j} jugador={j} comandantes={comandantes} limite={limite} />
								))}
							</tbody>
						</table>
					</ScrollArea>
				)}
			</DialogContent>
		</Dialog>
	);
}

function Punto({ jugador }: { jugador: IdJugador }) {
	const color = useMesa((s) => s.jugadores[jugador]?.color ?? "transparent");
	return <span className="size-2 shrink-0 rounded-full" style={{ background: color }} />;
}

function EncabezadoComandante({ c }: { c: Comandante }) {
	const { t } = useLang();
	const lanzado = useMesa((s) => s.jugadores[c.dueno]?.lanzamientos[c.id] ?? 0);
	const impuesto = useMesa((s) => impuestoDe(s.jugadores[c.dueno], c.id));
	return (
		<th className="rounded-md border bg-card/50 p-1.5 align-top font-normal">
			<div className="flex items-center gap-1 font-medium">
				<Punto jugador={c.dueno} />
				<span className="truncate">{c.nombre}</span>
			</div>
			<div className="mt-1 flex items-center justify-between gap-1 text-[10px] text-muted-foreground">
				<span>{t("Impuesto: {coste}", { coste: impuesto })}</span>
				<span className="flex items-center gap-0.5">
					<BotonPaso
						etiqueta={t("Una vez menos")}
						disabled={lanzado === 0}
						onClick={() => void hacer({ que: "lanzamientos", comandante: c.id, delta: -1 })}
					>
						<Icono icon="lucide:minus" className="size-3" />
					</BotonPaso>
					<b className="min-w-3 text-center tabular-nums">{lanzado}</b>
					<BotonPaso
						etiqueta={t("Una vez más")}
						onClick={() => void hacer({ que: "lanzamientos", comandante: c.id, delta: 1 })}
					>
						<Icono icon="lucide:plus" className="size-3" />
					</BotonPaso>
				</span>
			</div>
		</th>
	);
}

function FilaJugador({
	jugador,
	comandantes,
	limite,
}: {
	jugador: IdJugador;
	comandantes: Comandante[];
	limite: number;
}) {
	const nombre = useMesa((s) => s.jugadores[jugador]?.nombre ?? "");
	const fuera = useMesa((s) => s.jugadores[jugador]?.eliminado != null);
	return (
		<tr className={cn(fuera && "opacity-50")}>
			<th className="rounded-md border bg-card/50 p-1.5 text-left font-medium">
				<span className="flex items-center gap-1">
					<Punto jugador={jugador} />
					<span className="truncate">{nombre}</span>
				</span>
			</th>
			{comandantes.map((c) => (
				<Celda key={c.id} jugador={jugador} comandante={c} limite={limite} />
			))}
		</tr>
	);
}

function Celda({
	jugador,
	comandante,
	limite,
}: {
	jugador: IdJugador;
	comandante: Comandante;
	limite: number;
}) {
	const { t } = useLang();
	const dano = useMesa((s) => s.jugadores[jugador]?.danoComandante[comandante.id] ?? 0);
	const nombre = useMesa((s) => s.jugadores[jugador]?.nombre ?? "");
	// Un comandante no le hace daño de comandante a su dueño: la celda se apaga
	// (solo se puede quitar lo que ya hubiera, por si se apuntó de más).
	const propio = comandante.dueno === jugador;
	const mover = (delta: number) =>
		void hacer({
			que: "danoComandante",
			jugador,
			comandante: comandante.id as IdCarta,
			delta: deltaSeguro(dano, delta),
		});
	return (
		<td
			className={cn(
				"rounded-md border p-1 text-center",
				propio && "bg-muted/30 text-muted-foreground",
				dano >= limite && "border-rose-500 bg-rose-500/15 text-rose-400",
			)}
		>
			<span className="flex items-center justify-center gap-1">
				<BotonPaso
					etiqueta={t("Quitar daño de {comandante} a {jugador}", {
						comandante: comandante.nombre,
						jugador: nombre,
					})}
					disabled={dano === 0}
					onClick={() => mover(-1)}
				>
					<Icono icon="lucide:minus" className="size-3" />
				</BotonPaso>
				<b className="min-w-5 text-sm tabular-nums">{dano}</b>
				<BotonPaso
					etiqueta={t("Dar daño de {comandante} a {jugador}", {
						comandante: comandante.nombre,
						jugador: nombre,
					})}
					disabled={propio}
					onClick={() => mover(1)}
				>
					<Icono icon="lucide:plus" className="size-3" />
				</BotonPaso>
			</span>
		</td>
	);
}

function BotonPaso({
	etiqueta,
	disabled,
	onClick,
	children,
}: {
	etiqueta: string;
	disabled?: boolean;
	onClick: () => void;
	children: React.ReactNode;
}) {
	return (
		<button
			type="button"
			aria-label={etiqueta}
			disabled={disabled}
			onClick={onClick}
			className="flex size-5 items-center justify-center rounded-full border bg-background/50 hover:bg-accent disabled:opacity-30"
		>
			{children}
		</button>
	);
}
