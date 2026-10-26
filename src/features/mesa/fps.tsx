import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { useLang } from "@/components/language-provider";
import { useUi } from "@/stores/ui-store";

/** Por debajo de esto, la mesa va a trompicones. */
const FPS_MINIMO = 30;
/** Cuántos segundos seguidos por debajo antes de ofrecer bajar la calidad. */
const SEGUNDOS_LENTA = 120;

/** Llama a `alMedir` cada segundo con los fotogramas pintados en él. */
function useCadaSegundo(activo: boolean, alMedir: (fps: number) => void) {
	const medir = useRef(alMedir);
	medir.current = alMedir;
	useEffect(() => {
		if (!activo) return;
		let cuadros = 0;
		let desde = performance.now();
		let id = requestAnimationFrame(function paso(ahora) {
			cuadros++;
			if (ahora - desde >= 1000) {
				medir.current(Math.round((cuadros * 1000) / (ahora - desde)));
				cuadros = 0;
				desde = ahora;
			}
			id = requestAnimationFrame(paso);
		});
		return () => cancelAnimationFrame(id);
	}, [activo]);
}

/**
 * **Contador de fotogramas** (Ajustes > Mesa), en una esquina: cuántos
 * fotogramas pinta la vista web por segundo. Para ver si la mesa va fluida
 * con muchas cartas o con la calidad alta.
 */
export function ContadorFps() {
	const activo = useUi((s) => s.mesa.fps);
	const [fps, setFps] = useState(0);
	useCadaSegundo(activo, setFps);
	if (!activo) return null;
	return (
		<span className="pointer-events-none absolute bottom-1 left-1 z-[9000] rounded bg-black/70 px-1.5 font-mono text-[10px] text-lime-300 tabular-nums">
			{fps} fps
		</span>
	);
}

/**
 * Si la mesa va a menos de 30 fps durante 2 minutos seguidos y no está ya en
 * lo mínimo, ofrece el modo rendimiento (una vez por partida). Una ventana
 * oculta no pinta: esos segundos no cuentan.
 */
export function VigiaFps() {
	const { t } = useLang();
	const lentos = useRef(0);
	const avisado = useRef(false);
	useCadaSegundo(true, (fps) => {
		if (document.hidden || avisado.current) return;
		lentos.current = fps < FPS_MINIMO ? lentos.current + 1 : 0;
		if (lentos.current < SEGUNDOS_LENTA) return;
		const m = useUi.getState().mesa;
		if (m.calidad === 146 && !m.sombras && m.reducirAnimaciones) return;
		avisado.current = true;
		toast(t("La mesa va lenta en este equipo."), {
			duration: 15_000,
			action: {
				label: t("Bajar la calidad"),
				onClick: () => useUi.getState().setMesa(MODO_RENDIMIENTO),
			},
		});
	});
	return null;
}

/** Todo lo de la mesa al mínimo (Ajustes > Mesa > Modo rendimiento). */
export const MODO_RENDIMIENTO = {
	calidad: 146,
	sombras: false,
	reducirAnimaciones: true,
	panelEstado: false,
} as const;

/** Si hay que ahorrarse las animaciones: lo pide el ajuste o el sistema. */
export function useSinAnimaciones(): boolean {
	const ajuste = useUi((s) => s.mesa.reducirAnimaciones);
	const [sistema, setSistema] = useState(
		() => window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false,
	);
	useEffect(() => {
		const mq = window.matchMedia?.("(prefers-reduced-motion: reduce)");
		if (!mq) return;
		const cambia = () => setSistema(mq.matches);
		mq.addEventListener("change", cambia);
		return () => mq.removeEventListener("change", cambia);
	}, []);
	return ajuste || sistema;
}
