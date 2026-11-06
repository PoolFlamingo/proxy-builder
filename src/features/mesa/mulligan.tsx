import { useLayoutEffect, useRef, useState } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
import type { ModoMulligan } from "@/bindings/ModoMulligan";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { useFormato } from "@/lib/formato";
import { cn } from "@/lib/utils";
import { cartasDe, useMesa } from "@/stores/mesa-store";
import { hacer, zonaDe } from "./acciones";
import { useEncuentro } from "./encuentro";
import { useMesaMovil } from "./formato-mesa";
import { caraVisible, ImagenCarta } from "./imagen-carta";
import { manejadoresLeer } from "./leer-en-grande";
import { cartaEnFila } from "./movil/cuentas-mano";

/** Cuántas van al fondo al quedarse (lo mismo que el motor, `QuedarseCon`). */
export function alFondo(modo: ModoMulligan, mulligans: number, enMano: number): number | null {
	switch (modo) {
		case "londres":
			return Math.min(mulligans, enMano);
		case "londresPrimeroGratis":
			return Math.min(Math.max(0, mulligans - 1), enMano);
		case "amistoso":
			return 0;
		case "libre":
			return null;
	}
}

const REGLAS: Record<ModoMulligan, string> = {
	londres: paraTraducir("Londres: robas otra vez siete y pones al fondo una por cada mulligan."),
	londresPrimeroGratis: paraTraducir(
		"Londres con el primero gratis: el primer mulligan no cuesta ninguna carta.",
	),
	amistoso: paraTraducir("Amistoso: robas otra vez siete y no pones ninguna al fondo."),
	libre: paraTraducir("Libre: pones al fondo las que quieras."),
};

/**
 * **La mano inicial** (`capturas/pablo/02.jpg`): la mano en grande,
 * «Quedarse» o «Mulligan», con la regla elegida. Al quedarse, se eligen las
 * que van al fondo.
 */
export function Mulligan() {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const decidido = useMesa((s) => s.general?.mulligan?.decididos.includes(yo) ?? true);
	const hay = useMesa((s) => s.general?.mulligan != null);
	const mano = useMesa((s) => cartasDe(s, zonaDe(yo, "mano")));
	const cartas = useMesa((s) => s.cartas);
	const mulligans = useMesa((s) => s.jugadores[yo]?.mulligans ?? 0);
	const modo = useMesa((s) => s.opciones?.mulligan ?? "londres");
	const variosJugadores = useMesa((s) => s.asientos.length > 1);
	const primero = useMesa((s) =>
		s.general ? (s.jugadores[s.general.turno.primero]?.nombre ?? null) : null,
	);
	const colorPrimero = useMesa((s) =>
		s.general ? s.jugadores[s.general.turno.primero]?.color : undefined,
	);
	const encuentro = useEncuentro();
	const movil = useMesaMovil();
	const { horizontal } = useFormato();
	// Un teléfono de lado (~390 dp de alto): las cartas en una sola fila que se ajusta
	// al hueco que dejan el título y los botones, que se ven siempre.
	const enFila = movil && horizontal;
	const fila = useRef<HTMLDivElement>(null);
	const [hueco, setHueco] = useState({ w: 0, h: 0 });
	const abierto = hay && !decidido;
	useLayoutEffect(() => {
		const el = fila.current;
		if (!enFila || !abierto || !el) return;
		const medir = () => setHueco({ w: el.clientWidth, h: el.clientHeight });
		medir();
		const ro = new ResizeObserver(medir);
		ro.observe(el);
		return () => ro.disconnect();
	}, [enFila, abierto]);
	const [fondo, setFondo] = useState<IdCarta[]>([]);
	if (!abierto) return null;
	// Lo que baja la elegida (`translate-y-2`) también tiene que caber.
	const carta = cartaEnFila(hueco.w, Math.max(0, hueco.h - 10), mano.length);
	const toca = alFondo(modo, mulligans, mano.length);
	const listo = toca === null || fondo.length === toca;
	const alternar = (id: IdCarta) =>
		setFondo((f) => (f.includes(id) ? f.filter((x) => x !== id) : [...f, id]));
	return (
		<div
			className={cn(
				"absolute inset-0 z-[8500] flex flex-col items-center justify-center bg-black/70 backdrop-blur-sm",
				enFila ? "gap-1.5 px-2 py-1.5" : movil ? "gap-2 p-2" : "gap-4 p-6",
			)}
		>
			<div className="shrink-0 text-center text-white">
				<h2 className={cn("font-semibold", enFila ? "text-sm" : "text-lg")}>
					{t("Tu mano inicial")}
				</h2>
				<p className={cn("text-xs text-white/70", enFila && "line-clamp-1")}>{t(REGLAS[modo])}</p>
				{variosJugadores && primero && (
					<p
						className={cn(
							"mx-auto mt-1 flex w-fit max-w-full items-center gap-1.5 rounded-2xl bg-primary px-3 font-semibold text-primary-foreground shadow-lg",
							enFila ? "py-0.5 text-xs" : "py-1 text-sm",
						)}
					>
						<span
							className="size-2.5 shrink-0 rounded-full ring-1 ring-white/70"
							style={{ background: colorPrimero }}
						/>
						<Icono icon="lucide:flag" className="size-3.5 shrink-0" />
						<span className={cn("min-w-0", enFila && "truncate")}>
							{encuentro?.empieza?.porque === "perdio"
								? t("Empieza {quien}, que perdió la partida anterior", { quien: primero })
								: encuentro?.empieza?.porque === "elegido"
									? t("Empieza {quien}", { quien: primero })
									: t("Sorteado al azar: empieza {quien}", { quien: primero })}
						</span>
					</p>
				)}
				{mulligans > 0 && (
					<p className="text-xs text-white/70">{t("Mulligans: {n}", { n: mulligans })}</p>
				)}
				{toca !== null && toca > 0 && (
					<p className="mt-1 text-xs text-amber-300">
						{t("Elige {n} para poner al fondo ({hechas} elegidas).", {
							n: toca,
							hechas: fondo.length,
						})}
					</p>
				)}
			</div>
			<div
				ref={fila}
				className={cn(
					"flex max-w-full justify-center",
					enFila ? "min-h-0 w-full flex-1 flex-nowrap items-center gap-1.5" : "flex-wrap gap-2.5",
				)}
			>
				{mano.map((id) => {
					const c = cartas[id];
					if (!c) return null;
					const elegida = fondo.includes(id);
					return (
						<button
							key={id}
							type="button"
							onClick={() => toca !== 0 && alternar(id)}
							{...manejadoresLeer(id)}
							className={cn(
								"relative overflow-hidden rounded-[5%] shadow-xl transition",
								// En un móvil de lado (411 dp de alto) la mano entera tiene que caber con sus botones.
								enFila ? "shrink-0" : movil ? "h-36 w-[103px]" : "h-60 w-[172px]",
								elegida && "opacity-50 ring-4 ring-amber-400",
								elegida && (enFila ? "translate-y-2" : "translate-y-3"),
							)}
							style={enFila ? { width: carta.w, height: carta.h } : undefined}
						>
							<ImagenCarta imagen={caraVisible(c)} ancho={244} nombre={c.origen?.nombre} />
							{elegida && (
								<span className="absolute inset-x-0 bottom-2 text-center text-xs font-semibold text-amber-300">
									{t("Al fondo")}
								</span>
							)}
						</button>
					);
				})}
			</div>
			<div className="flex shrink-0 gap-2.5">
				<Button
					variant="outline"
					onClick={() => {
						setFondo([]);
						void hacer({ que: "mulligan" });
					}}
				>
					<Icono icon="lucide:refresh-cw" className="size-4" />
					{t("Mulligan")}
				</Button>
				<Button
					disabled={!listo}
					onClick={() => {
						void hacer({ que: "quedarseCon", alFondo: fondo }).then(() => setFondo([]));
					}}
				>
					<Icono icon="lucide:check" className="size-4" />
					{t("Quedarse")}
				</Button>
			</div>
		</div>
	);
}
