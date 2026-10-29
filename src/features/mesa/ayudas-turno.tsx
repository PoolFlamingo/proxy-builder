import { useEffect, useRef, useState } from "react";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { useMesa } from "@/stores/mesa-store";
import { hacer } from "./acciones";
import * as op from "./operaciones";

/** Lo que duran los botones del principio del turno, en ms. */
const DURAN_MS = 10_000;

type Boton = "enderezar" | "robar" | "rad";

/**
 * **Las ayudas de la mesa en pantalla** (no son reglas):
 *
 * - al empezar mi turno, si las ayudas de enderezar o robar están apagadas,
 *   salen sus botones (en ese orden) durante 10 s; con contadores de rad,
 *   también el recordatorio de moler;
 * - el monarca tiene siempre a mano su «Robar».
 *
 * Los avisos de vida a 0, veneno y comandante los da `muerte.tsx`, con lo
 * que dice Rust (`fueraDeJuego`).
 */
export function AyudasTurno() {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const numero = useMesa((s) => s.general?.turno.numero ?? 0);
	const activo = useMesa((s) => s.general?.turno.activo === yo);
	const ayudas = useMesa((s) => s.opciones?.ayudas);
	const rad = useMesa((s) => s.jugadores[yo]?.contadores.rad ?? 0);
	const monarca = useMesa((s) => s.general?.designaciones.monarca === yo);
	const fin = useMesa((s) => s.general?.fin != null);
	const [botones, setBotones] = useState<Boton[]>([]);
	const turnoVisto = useRef(0);

	useEffect(() => {
		if (!activo || numero === 0 || turnoVisto.current === numero || !ayudas) return;
		turnoVisto.current = numero;
		const lista: Boton[] = [];
		if (!ayudas.enderezarAlEmpezar) lista.push("enderezar");
		if (!ayudas.robarAlEmpezar) lista.push("robar");
		if (rad > 0) lista.push("rad");
		setBotones(lista);
	}, [activo, numero, ayudas, rad]);

	useEffect(() => {
		if (botones.length === 0) return;
		const id = window.setTimeout(() => setBotones([]), DURAN_MS);
		return () => window.clearTimeout(id);
	}, [botones]);

	const quitar = (b: Boton) => setBotones((l) => l.filter((x) => x !== b));
	if (fin || (botones.length === 0 && !monarca)) return null;
	return (
		<div className="flex flex-col items-end gap-1.5">
			{botones.map((b) => (
				<Button
					key={b}
					size="sm"
					variant="secondary"
					className="animate-in rounded-full shadow-lg fade-in slide-in-from-right-4"
					onClick={() => {
						quitar(b);
						if (b === "enderezar") void op.enderezarTodo();
						else if (b === "robar") void op.robar();
						else void op.moler(rad);
					}}
				>
					{b === "enderezar" ? (
						<>
							<Icono icon="lucide:rotate-ccw" className="size-3.5" />
							{t("Enderezar todo")}
						</>
					) : b === "robar" ? (
						<>
							<Icono icon="lucide:hand" className="size-3.5" />
							{t("Robar")}
						</>
					) : (
						<>
							<Icono icon="game-icons:tombstone" className="size-3.5" />
							{t("Rad: moler {n}", { n: rad })}
						</>
					)}
				</Button>
			))}
			{monarca && (
				<Button
					size="sm"
					variant="secondary"
					className="rounded-full shadow-lg"
					onClick={() => void hacer({ que: "robar", n: 1 })}
				>
					<Icono icon="game-icons:crown" className="size-3.5 text-amber-400" />
					{t("Robar (monarca)")}
				</Button>
			)}
		</div>
	);
}
