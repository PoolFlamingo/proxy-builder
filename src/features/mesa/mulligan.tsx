import { Check, RefreshCw } from "lucide-react";
import { useState } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
import type { ModoMulligan } from "@/bindings/ModoMulligan";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { cartasDe, useMesa } from "@/stores/mesa-store";
import { hacer, zonaDe } from "./acciones";
import { useEncuentro } from "./encuentro";
import { caraVisible, ImagenCarta } from "./imagen-carta";

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
	const encuentro = useEncuentro();
	const [fondo, setFondo] = useState<IdCarta[]>([]);
	if (!hay || decidido) return null;
	const toca = alFondo(modo, mulligans, mano.length);
	const listo = toca === null || fondo.length === toca;
	const alternar = (id: IdCarta) =>
		setFondo((f) => (f.includes(id) ? f.filter((x) => x !== id) : [...f, id]));
	return (
		<div className="absolute inset-0 z-[8500] flex flex-col items-center justify-center gap-4 bg-black/70 p-6 backdrop-blur-sm">
			<div className="text-center text-white">
				<h2 className="text-lg font-semibold">{t("Tu mano inicial")}</h2>
				<p className="text-xs text-white/70">{t(REGLAS[modo])}</p>
				{variosJugadores && primero && (
					<p className="mt-1 text-sm font-semibold text-primary">
						{encuentro?.empieza?.porque === "perdio"
							? t("Empieza {quien}, que perdió la partida anterior", { quien: primero })
							: encuentro?.empieza?.porque === "elegido"
								? t("Empieza {quien}", { quien: primero })
								: t("Sorteado al azar: empieza {quien}", { quien: primero })}
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
			<div className="flex max-w-full flex-wrap justify-center gap-2.5">
				{mano.map((id) => {
					const c = cartas[id];
					if (!c) return null;
					const elegida = fondo.includes(id);
					return (
						<button
							key={id}
							type="button"
							onClick={() => toca !== 0 && alternar(id)}
							onPointerEnter={() => useMesa.getState().setSobre(id)}
							onPointerLeave={() => useMesa.getState().setSobre(null)}
							className={cn(
								"relative h-60 w-[172px] overflow-hidden rounded-[5%] shadow-xl transition",
								elegida && "translate-y-3 opacity-50 ring-4 ring-amber-400",
							)}
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
			<div className="flex gap-2.5">
				<Button
					variant="outline"
					onClick={() => {
						setFondo([]);
						void hacer({ que: "mulligan" });
					}}
				>
					<RefreshCw className="size-4" />
					{t("Mulligan")}
				</Button>
				<Button
					disabled={!listo}
					onClick={() => {
						void hacer({ que: "quedarseCon", alFondo: fondo }).then(() => setFondo([]));
					}}
				>
					<Check className="size-4" />
					{t("Quedarse")}
				</Button>
			</div>
		</div>
	);
}
