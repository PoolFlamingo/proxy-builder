import { useState } from "react";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { useFormato } from "@/lib/formato";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { useRepeticion } from "@/stores/repeticion-store";
import * as op from "../operaciones";

/**
 * **«Pasar turno»** del móvil: un botón redondo arriba a la derecha, solo en
 * mi turno (no en el mulligan, ni mirando, ni en una repetición). Es el mismo
 * `Accion::PasarTurno` de siempre.
 */
export function BotonPasarTurno() {
	const { t } = useLang();
	const esMiTurno = useMesa((s) => s.yo !== null && s.general?.turno.activo === s.yo);
	const mulligan = useMesa((s) => s.general?.mulligan != null);
	const mirando = useRed((s) => s.papel === "espectador");
	const repitiendo = useRepeticion((s) => s.activa !== null);
	if (!esMiTurno || mulligan || mirando || repitiendo) return null;
	return (
		<Button
			size="icon"
			onClick={() => void op.pasarTurno()}
			aria-label={t("Pasar turno")}
			// Del alto de la barra de arriba (que es compacta): más grande se comía el campo.
			className="absolute top-1 right-1 z-[7200] size-11 rounded-full shadow-xl"
			data-tutorial="mesa-pasar"
		>
			<Icono icon="lucide:skip-forward" className="size-5" />
		</Button>
	);
}

/**
 * **«Gira el teléfono»**: la mesa se juega en horizontal. En vertical sale un
 * aviso encima, que se quita con un toque; la orientación no se bloquea (el
 * resto de la app sí va en vertical). Durante el mulligan no sale: la mano inicial se
 * ve bien de pie.
 */
export function AvisoGirar() {
	const { t } = useLang();
	const { horizontal } = useFormato();
	const mulligan = useMesa((s) => s.general?.mulligan != null);
	const [oculto, setOculto] = useState(false);
	if (horizontal || oculto || mulligan) return null;
	return (
		<div className="absolute inset-x-3 top-1/3 z-[9500] flex flex-col items-center gap-3 rounded-2xl border border-white/15 bg-black/85 p-5 text-center text-white shadow-2xl">
			<Icono icon="lucide:smartphone" className="size-12 rotate-90" />
			<p className="text-base font-semibold">{t("Gira el teléfono")}</p>
			<p className="text-xs text-white/70">
				{t("La mesa se juega en horizontal: se ve mejor y caben las manos y el campo.")}
			</p>
			<Button variant="secondary" onClick={() => setOculto(true)}>
				{t("Seguir en vertical")}
			</Button>
		</div>
	);
}
