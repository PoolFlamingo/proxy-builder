import { useEffect, useRef } from "react";
import { toast } from "sonner";
import { useShallow } from "zustand/react/shallow";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { cartasDe, cuantasEn, useMesa } from "@/stores/mesa-store";
import { hacer, zonaDe } from "./acciones";
import { caraVisible, ImagenCarta } from "./imagen-carta";
import { esArchienemigo } from "./planar";

/**
 * **Los planes del archienemigo** (Archenemy): cuántos le quedan en el mazo,
 * los que tiene en marcha y los botones de ponerlo en marcha y abandonarlo,
 * que son solo suyos. Al empezar su turno, un aviso le recuerda poner uno
 * en marcha.
 */
export function PanelPlanes() {
	const archenemy = useMesa((s) => s.variantes?.archenemy ?? null);
	return archenemy === null ? null : <Planes archenemy={archenemy} />;
}

function Planes({ archenemy }: { archenemy: number }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo);
	const mio = esArchienemigo(archenemy, yo);
	const enMazo = useMesa((s) => cuantasEn(s, zonaDe(archenemy, "planes")));
	const enMarcha = useMesa(useShallow((s) => cartasDe(s, zonaDe(archenemy, "planesEnMarcha"))));
	const turno = useMesa((s) => s.general?.turno);
	const toca = turno?.activo === archenemy && (turno?.numero ?? 0) > 0;
	const avisado = useRef(-1);
	useEffect(() => {
		if (!mio || !toca || enMazo === 0 || !turno || avisado.current === turno.numero) return;
		avisado.current = turno.numero;
		toast.info(t("Es tu turno de archienemigo: pon en marcha un plan."), {
			action: {
				label: t("Poner en marcha"),
				onClick: () => void hacer({ que: "ponerEnMarcha" }),
			},
		});
	}, [mio, toca, enMazo, turno, t]);
	if (enMazo === 0 && enMarcha.length === 0) return null;
	return (
		<div
			className="flex w-48 flex-col gap-1.5 rounded-lg border border-white/10 bg-black/60 p-2 text-white shadow-lg backdrop-blur-md"
			data-tutorial="mesa-planes"
		>
			<div className="flex flex-col text-[10px] leading-tight">
				<span className="font-semibold tracking-wider uppercase">{t("Planes en marcha")}</span>
				<span className="text-white/60">{t("{cuantos} en el mazo", { cuantos: enMazo })}</span>
			</div>
			<ul className="flex max-h-44 flex-col gap-1.5">
				{enMarcha.map((id) => (
					<Plan key={id} id={id} mio={mio} />
				))}
			</ul>
			<Button
				size="sm"
				disabled={!mio || enMazo === 0}
				onClick={() => void hacer({ que: "ponerEnMarcha" })}
			>
				{t("Poner en marcha")}
			</Button>
		</div>
	);
}

function Plan({ id, mio }: { id: number; mio: boolean }) {
	const { t } = useLang();
	const c = useMesa((s) => s.cartas[id]);
	if (!c) return null;
	return (
		<li className="flex items-center gap-1.5">
			<button
				type="button"
				className="aspect-[88/63] w-20 shrink-0 overflow-hidden rounded border border-white/15"
				onPointerEnter={() => useMesa.getState().setSobre(id)}
				onPointerLeave={() => {
					if (useMesa.getState().sobre === id) useMesa.getState().setSobre(null);
				}}
				onClick={() => useMesa.getState().inspeccionar(id)}
				aria-label={c.origen?.nombre ?? t("Plan")}
			>
				<ImagenCarta imagen={caraVisible(c)} ancho={146} nombre={c.origen?.nombre} />
			</button>
			<Button
				size="sm"
				variant="ghost"
				className="h-6 px-1.5 text-[11px] text-white hover:bg-white/10 hover:text-white"
				disabled={!mio}
				onClick={() => void hacer({ que: "abandonarPlan", carta: id })}
			>
				{t("Abandonar")}
			</Button>
		</li>
	);
}
