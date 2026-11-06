import { useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { PapelRed } from "@/bindings/PapelRed";
import { useLang } from "@/components/language-provider";
import { cn } from "@/lib/utils";
import { type Mesa, tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";

/** Lo que dura el aviso de «turno de…» en el centro, en ms (lo mismo que su animación). */
const DURA_AVISO_MS = 1800;

/** De quién se está pendiente: quien tiene el turno o, antes del primero, quien empezará. */
export type QuienJuega = {
	id: number;
	nombre: string;
	color: string;
	/** Aún no ha empezado el primer turno. */
	empezando: boolean;
	mio: boolean;
};

/**
 * Quién juega, o `null` si no hay nada que señalar: en solitario es siempre uno,
 * y con la partida terminada ya no es de nadie. «Mío» solo existe en red: en una
 * partida local, el «yo» es el jugador que se mira, y en todo caso se dice el nombre.
 */
export function quienJuega(
	m: Pick<Mesa, "general" | "jugadores" | "asientos" | "yo">,
	papel: PapelRed | null,
): QuienJuega | null {
	const g = m.general;
	if (!g || m.asientos.length < 2 || g.fin != null) return null;
	const empezando = g.mulligan != null || g.turno.numero === 0;
	const id = empezando ? g.turno.primero : g.turno.activo;
	const j = m.jugadores[id];
	if (!j) return null;
	const equipo = j.equipo != null ? g.equipos.find((e) => e.id === j.equipo) : undefined;
	return {
		id,
		nombre: j.nombre,
		color: equipo?.color || j.color,
		empezando,
		mio: papel !== null && papel !== "espectador" && m.yo === id,
	};
}

function useQuienJuega(): QuienJuega | null {
	const papel = useRed((s) => s.papel);
	return useMesa(useShallow((s) => quienJuega(s, papel)));
}

/** La frase: «Tu turno», «Turno de Ana», «Empiezas tú», «Empieza Ana». */
function useTextoTurno(): (q: QuienJuega) => string {
	const { t } = useLang();
	return (q) =>
		q.empezando
			? q.mio
				? t("Empiezas tú")
				: t("Empieza {quien}", { quien: q.nombre })
			: q.mio
				? t("Tu turno")
				: t("Turno de {quien}", { quien: q.nombre });
}

/**
 * **De quién es el turno**, en la barra de la mesa: con el color del jugador, y
 * en el color de la app cuando te toca a ti. `compacto`: la barra del móvil, sin
 * fondo salvo en tu turno, para que quepa encima del número de turno.
 */
export function ChipTurno({ compacto = false }: { compacto?: boolean }) {
	const q = useQuienJuega();
	const texto = useTextoTurno();
	if (!q) return null;
	return (
		<span
			className={cn(
				"flex max-w-44 min-w-0 items-center font-semibold whitespace-nowrap",
				compacto
					? "gap-1 self-start rounded-sm text-[11px]"
					: "gap-1.5 rounded-md px-1.5 py-0.5 text-xs",
				q.mio ? "bg-primary px-1 text-primary-foreground" : !compacto && "bg-white/10",
			)}
		>
			<span
				className={cn(
					"shrink-0 rounded-full ring-1 ring-white/60",
					compacto ? "size-2" : "size-2.5",
				)}
				style={{ background: q.color }}
			/>
			<span className="min-w-0 truncate">{texto(q)}</span>
		</span>
	);
}

/**
 * **Cambia el turno**: la frase en grande un momento, en mitad de la mesa. No
 * recibe el puntero (no estorba si se está jugando una carta); no sale al abrir la
 * mesa ni al deshacer, solo cuando el turno avanza.
 */
export function AvisoTurno() {
	const numero = useMesa((s) => s.general?.turno.numero ?? 0);
	const q = useQuienJuega();
	const texto = useTextoTurno();
	const visto = useRef(numero);
	const [aviso, setAviso] = useState<number | null>(null);
	useEffect(() => {
		const antes = visto.current;
		visto.current = numero;
		if (numero <= antes) {
			setAviso(null);
			return;
		}
		setAviso(numero);
		const id = window.setTimeout(() => setAviso(null), DURA_AVISO_MS);
		return () => window.clearTimeout(id);
	}, [numero]);
	if (aviso === null || !q || q.empezando) return null;
	return (
		<div className="pointer-events-none absolute inset-x-2 top-[18%] z-[7100] flex justify-center">
			<div
				key={aviso}
				className="max-w-full animate-[aviso-turno_1.8s_ease-out_forwards] truncate rounded-full border-2 bg-black/80 px-5 py-2 text-base font-semibold sm:text-lg text-white shadow-2xl"
				style={{ borderColor: q.mio ? "var(--primary)" : q.color }}
			>
				{texto(q)}
			</div>
		</div>
	);
}

/**
 * **El aro del tablero** del jugador al que le toca, encima de todo lo del tablero
 * y sin recibir el puntero. Sin difuminado: está sobre cartas que se mueven y un
 * difuminado grande habría que repintarlo cada vez.
 */
export function AroTurno() {
	const viendo = useMesa(tableroDe);
	const q = useQuienJuega();
	if (!q || q.empezando || q.id !== viendo) return null;
	return (
		<div
			className="pointer-events-none absolute inset-0 z-[6600] rounded-md"
			style={{
				boxShadow:
					"inset 0 0 0 3px var(--primary), inset 0 0 0 6px color-mix(in srgb, var(--primary) 30%, transparent)",
			}}
		/>
	);
}
