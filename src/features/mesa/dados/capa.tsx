import { useEffect, useMemo, useRef, useState } from "react";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { type NivelDados, useUi } from "@/stores/ui-store";
import { ResultadoAzar } from "../dialogos/azar";
import { useSinAnimaciones } from "../fps";
import { claveEstilo, estiloDe, useAspectoDados } from "./aspecto";
import { type Tirada, useDados } from "./cola";
import { DadosCss } from "./css3d";
import type { EscenaDados } from "./escena3d";
import { DadosPlanos } from "./plano";
import { golpeDado, golpeMoneda, lanzarMoneda } from "./sonidos";
import { lienzoFondo, urlFondo } from "./texturas";

/** Lo que se queda el resultado en pantalla, en ms. */
const RESULTADO_MS = 2200;

/**
 * Lo más que se espera a que una tirada acabe de animarse, en ms (la física
 * dura como mucho 4 s, más la carga de three y Rapier la primera vez). Pasado
 * esto sale el resultado igual: una animación que no termina no puede dejar
 * la cola parada y sin dados el resto de la partida.
 */
const TOPE_TIRADA_MS = 8000;

/** Si hay WebGL2 (sin él, los dados van en CSS 3D). Se mira una vez. */
let webgl2: boolean | null = null;
function hayWebGL2(): boolean {
	if (webgl2 === null) {
		try {
			webgl2 = !!document.createElement("canvas").getContext("webgl2");
		} catch {
			webgl2 = false;
		}
	}
	return webgl2;
}

/** El nivel que se usa: el de Ajustes, o el mejor que aguante el equipo. */
function nivelReal(pedido: NivelDados, fallo3d: boolean): Exclude<NivelDados, "auto"> {
	if (pedido === "auto") return hayWebGL2() && !fallo3d ? "3d" : "css";
	if (pedido === "3d" && fallo3d) return "css";
	return pedido;
}

/**
 * **La capa de los dados**, encima del tablero: enseña cada tirada que llega
 * (`useDados`) con el nivel elegido —3D con three.js y física, CSS 3D low
 * poly o 2D— y luego el resultado en grande. No capta el ratón salvo el
 * cartel del resultado (un clic lo quita). Con las animaciones reducidas
 * sale solo el resultado.
 */
export function CapaDados() {
	const tirada = useDados((s) => s.tiradas[0] ?? null);
	const pedido = useUi((s) => s.mesa.dados);
	const [fallo3d, setFallo3d] = useState(false);
	const sinAnimaciones = useSinAnimaciones();
	const nivel = sinAnimaciones ? null : nivelReal(pedido, fallo3d);
	const [parada, setParada] = useState<number | null>(null);
	const lienzo = useRef<HTMLCanvasElement>(null);
	const escena = useRef<EscenaDados | null>(null);
	// El 3D no acabó una tirada: desde la siguiente, CSS 3D.
	const colgado3d = useRef(false);
	const color = useMesa((s) =>
		tirada?.actor != null ? (s.jugadores[tirada.actor]?.color ?? "#7c3aed") : "#7c3aed",
	);
	const yo = useMesa((s) => s.yo);
	const aspecto = useAspectoDados((s) => s.vista);
	useEffect(() => {
		void useAspectoDados.getState().cargar();
	}, []);
	// Con los dados en 3D, three y Rapier se cargan en un rato libre al abrir la
	// mesa: la primera tirada no se queda parada esperándolos.
	useEffect(() => {
		if (nivel !== "3d") return;
		const pronto = () => {
			void import("./escena3d").catch(() => {});
			void import("./fisica").then((m) => m.cargarRapier()).catch(() => {});
		};
		const ric = (window as { requestIdleCallback?: (f: () => void) => number }).requestIdleCallback;
		const id = ric ? ric(pronto) : window.setTimeout(pronto, 1500);
		return () => {
			if (!ric) window.clearTimeout(id);
		};
	}, [nivel]);
	const estilo = useMemo(
		() => estiloDe(aspecto, color, tirada !== null && tirada.actor === yo),
		[aspecto, color, tirada, yo],
	);
	// El fondo de las caras se pinta una vez por estilo (con imagen, al cargarla).
	const [fondo, setFondo] = useState<{
		clave: string;
		lienzo: HTMLCanvasElement;
		url: string;
	} | null>(null);
	const clave = claveEstilo(estilo);
	useEffect(() => {
		let vivo = true;
		void lienzoFondo(estilo).then((lienzo) => {
			// El mismo estilo no cambia nada (si no, el 3D volvería a tirar).
			if (vivo)
				setFondo((f) => (f?.clave === clave ? f : { clave, lienzo, url: urlFondo(lienzo) }));
		});
		return () => {
			vivo = false;
		};
	}, [estilo, clave]);
	const fondoListo = fondo?.clave === clave ? fondo : null;

	// Sin animaciones, directamente el resultado.
	useEffect(() => {
		if (tirada && nivel === null) setParada(tirada.id);
	}, [tirada, nivel]);

	// Si la animación no acaba (el 3D colgado o sin pintar), el resultado igual,
	// y las siguientes en CSS 3D.
	useEffect(() => {
		if (!tirada || parada === tirada.id) return;
		const id = window.setTimeout(() => {
			if (nivel === "3d") {
				escena.current?.recoger();
				colgado3d.current = true;
			}
			setParada(tirada.id);
		}, TOPE_TIRADA_MS);
		return () => window.clearTimeout(id);
	}, [tirada, parada, nivel]);

	// Al acabar (y tras enseñar el resultado), la siguiente.
	useEffect(() => {
		if (!tirada || parada !== tirada.id) return;
		const id = window.setTimeout(() => {
			escena.current?.recoger();
			if (colgado3d.current) setFallo3d(true);
			useDados.getState().siguiente();
		}, RESULTADO_MS);
		return () => window.clearTimeout(id);
	}, [tirada, parada]);

	// El 3D: la escena se crea la primera vez (three y Rapier van aparte).
	useEffect(() => {
		if (nivel !== "3d" || !tirada || !lienzo.current || !fondoListo) return;
		let vivo = true;
		const canvas = lienzo.current;
		void (async () => {
			try {
				if (!escena.current) {
					const { EscenaDados } = await import("./escena3d");
					escena.current = new EscenaDados(canvas);
				}
				if (tirada.piezas.some((p) => p.forma === "moneda")) lanzarMoneda();
				await escena.current.tirar(tirada, estilo, fondoListo.lienzo, (moneda, fuerza) =>
					moneda ? golpeMoneda(fuerza) : golpeDado(fuerza),
				);
				if (vivo) setParada(tirada.id);
			} catch {
				// Sin WebGL o sin WebAssembly: CSS 3D desde ahora.
				escena.current = null;
				if (vivo) setFallo3d(true);
			}
		})();
		return () => {
			vivo = false;
		};
	}, [nivel, tirada, estilo, fondoListo]);

	useEffect(() => {
		const medir = () => escena.current?.medir();
		window.addEventListener("resize", medir);
		return () => {
			window.removeEventListener("resize", medir);
			escena.current?.destruir();
			escena.current = null;
		};
	}, []);

	const acabar = (t: Tirada) => () => setParada(t.id);
	return (
		// Entre la barra de arriba y la mano: ahí cabe la mesa de la física.
		<div className="pointer-events-none absolute inset-x-0 top-[11%] bottom-[26%] z-[7200]">
			<canvas
				ref={lienzo}
				className={cn("absolute inset-0 size-full", nivel !== "3d" && "hidden")}
			/>
			{tirada && fondoListo && nivel === "css" && (
				<DadosCss
					key={tirada.id}
					tirada={tirada}
					estilo={estilo}
					fondo={fondoListo.url}
					alAcabar={acabar(tirada)}
				/>
			)}
			{tirada && fondoListo && nivel === "2d" && (
				<DadosPlanos
					key={tirada.id}
					tirada={tirada}
					estilo={estilo}
					fondo={fondoListo.url}
					alAcabar={acabar(tirada)}
				/>
			)}
			{tirada && parada === tirada.id && (
				<button
					type="button"
					onClick={() => {
						escena.current?.recoger();
						if (colgado3d.current) setFallo3d(true);
						useDados.getState().siguiente();
					}}
					className="pointer-events-auto absolute -bottom-2 left-1/2 w-72 -translate-x-1/2 animate-in rounded-xl border bg-background/95 p-1 shadow-2xl fade-in zoom-in-95"
				>
					<ResultadoAzar cambio={tirada.cambio} />
				</button>
			)}
		</div>
	);
}
