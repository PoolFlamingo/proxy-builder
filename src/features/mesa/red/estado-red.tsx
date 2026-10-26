import { Eye, Heart, Loader2, WifiOff } from "lucide-react";
import { useShallow } from "zustand/react/shallow";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { salir } from "../conexion";

/**
 * **Los jugadores de la mesa** (con más de uno): su color, su nombre y su
 * vida; pulsar uno enseña su tablero. Los tableros de todos a la vez
 * llegan con la Fase G.
 */
export function SelectorTablero() {
	const { t } = useLang();
	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const yo = useMesa((s) => s.yo);
	const viendo = useMesa(tableroDe);
	const espectador = useRed((s) => s.papel === "espectador");
	if (asientos.length < 2) return null;
	return (
		<div className="absolute top-2 left-2 z-[7000] flex flex-col gap-1 rounded-xl border border-white/10 bg-black/60 p-1 text-white shadow-lg backdrop-blur-md">
			{espectador && (
				<span className="flex items-center gap-1 px-1.5 text-[10px] text-white/60">
					<Eye className="size-3" />
					{t("Mirando la partida")}
				</span>
			)}
			{asientos.map((id) => {
				const j = jugadores[id];
				if (!j) return null;
				return (
					<button
						key={id}
						type="button"
						onClick={() => useMesa.getState().mirar(id)}
						className={cn(
							"flex items-center gap-1.5 rounded-lg px-1.5 py-1 text-left text-xs transition hover:bg-white/10",
							viendo === id && "bg-white/15",
							j.eliminado && "opacity-50",
						)}
					>
						<span className="size-2.5 shrink-0 rounded-full" style={{ background: j.color }} />
						<span className="max-w-32 truncate font-medium">
							{id === yo ? t("{nombre} (tú)", { nombre: j.nombre }) : j.nombre}
						</span>
						<span className="ml-auto flex items-center gap-0.5 pl-1.5 text-white/70 tabular-nums">
							<Heart className="size-3" />
							{j.vida}
						</span>
					</button>
				);
			})}
		</div>
	);
}

/** Si la conexión con el anfitrión se corta: «Reconectando…» o perdida. */
export function AvisoConexion() {
	const { t } = useLang();
	const conexion = useRed((s) => s.conexion);
	const intento = useRed((s) => s.intento);
	if (conexion === "conectada") return null;
	return (
		<div className="absolute top-14 left-1/2 z-[7500] flex -translate-x-1/2 items-center gap-2.5 rounded-xl border border-amber-500/40 bg-black/80 px-2.5 py-1.5 text-xs text-white shadow-lg backdrop-blur-md">
			{conexion === "reconectando" ? (
				<>
					<Loader2 className="size-3.5 animate-spin text-amber-400" />
					{t("Reconectando con el anfitrión… (intento {intento} de 5)", { intento })}
				</>
			) : (
				<>
					<WifiOff className="size-3.5 text-red-400" />
					{t("Se ha perdido la conexión con la partida.")}
					<Button size="sm" variant="secondary" onClick={() => void salir(false)}>
						{t("Salir")}
					</Button>
				</>
			)}
		</div>
	);
}
