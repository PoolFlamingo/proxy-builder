import { ChevronLeft, ChevronRight, X } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { useUi, type Vista } from "@/stores/ui-store";
import { PASOS } from "./pasos";

/** Ancho de la tarjeta y distancias a la zona y a los bordes (px). */
const ANCHO = 320;
const HUECO = 14;
const MARGEN = 16;
/** Lo que el foco sobresale de la zona (px). */
const AIRE = 6;

type Rect = { x: number; y: number; w: number; h: number };

/** El rectángulo de la zona en pantalla, si está y se ve. */
function zona(ancla: string | undefined): Rect | null {
	if (!ancla) return null;
	const el = document.querySelector(`[data-tutorial="${ancla}"]`);
	if (!el) return null;
	const r = el.getBoundingClientRect();
	if (r.width < 4 || r.height < 4) return null;
	return { x: r.left, y: r.top, w: r.width, h: r.height };
}

/**
 * Dónde va la tarjeta: a la derecha de la zona, a la izquierda, debajo o
 * encima (lo primero que quepa), y si no cabe en ningún lado, en el centro.
 */
function sitioTarjeta(r: Rect | null, alto: number): { left: number; top: number } {
	const vw = window.innerWidth;
	const vh = window.innerHeight;
	const centro = { left: (vw - ANCHO) / 2, top: Math.max(MARGEN, (vh - alto) / 2) };
	if (!r) return centro;
	const enY = (y: number) => Math.min(Math.max(MARGEN, y), vh - alto - MARGEN);
	const enX = (x: number) => Math.min(Math.max(MARGEN, x), vw - ANCHO - MARGEN);
	if (r.x + r.w + HUECO + ANCHO + MARGEN <= vw) return { left: r.x + r.w + HUECO, top: enY(r.y) };
	if (r.x - HUECO - ANCHO >= MARGEN) return { left: r.x - HUECO - ANCHO, top: enY(r.y) };
	if (r.y + r.h + HUECO + alto + MARGEN <= vh) return { left: enX(r.x), top: r.y + r.h + HUECO };
	if (r.y - HUECO - alto >= MARGEN) return { left: enX(r.x), top: r.y - HUECO - alto };
	return centro;
}

const cerrar = () => useUi.getState().abrirTutorial(null);

/**
 * **El tutorial de una pantalla**: oscurece todo menos la zona de cada paso
 * y, al lado, una tarjeta que cuenta qué se puede hacer en ella. Sale solo la
 * primera vez que se entra en cada pantalla (`useTutorialInicial`) y el botón
 * de ayuda de la barra lo vuelve a abrir. Flechas o Intro para avanzar, Esc
 * para cerrar.
 */
export function Tutorial() {
	const vista = useUi((s) => s.tutorial);
	if (!vista) return null;
	return <Recorrido key={vista} vista={vista} />;
}

function Recorrido({ vista }: { vista: Vista }) {
	const { t } = useLang();
	const pasos = PASOS[vista];
	const [i, setI] = useState(0);
	const [rect, setRect] = useState<Rect | null>(null);
	const [alto, setAlto] = useState(200);
	const tarjeta = useRef<HTMLDivElement>(null);
	const siguiente = useRef<HTMLButtonElement>(null);
	const paso = pasos[i];
	const ultimo = i === pasos.length - 1;

	// La zona se vuelve a medir al cambiar de paso, de tamaño o al moverse
	// algo (los paneles se pueden redimensionar).
	useLayoutEffect(() => {
		const medir = () => setRect(zona(paso?.ancla));
		medir();
		const el = paso?.ancla ? document.querySelector(`[data-tutorial="${paso.ancla}"]`) : null;
		const ro = new ResizeObserver(medir);
		if (el) ro.observe(el);
		window.addEventListener("resize", medir);
		return () => {
			ro.disconnect();
			window.removeEventListener("resize", medir);
		};
	}, [paso?.ancla]);

	useLayoutEffect(() => {
		if (tarjeta.current) setAlto(tarjeta.current.offsetHeight);
	});

	useEffect(() => {
		siguiente.current?.focus();
	}, [i]);

	useEffect(() => {
		// En captura y sin dejar pasar: con el tutorial abierto, las teclas
		// no llegan a la pantalla de debajo.
		const tecla = (e: KeyboardEvent) => {
			if (e.key === "Escape") cerrar();
			else if (e.key === "ArrowRight") {
				if (ultimo) cerrar();
				else setI((n) => n + 1);
			} else if (e.key === "ArrowLeft") setI((n) => Math.max(0, n - 1));
			else return;
			e.preventDefault();
			e.stopPropagation();
		};
		window.addEventListener("keydown", tecla, true);
		return () => window.removeEventListener("keydown", tecla, true);
	}, [ultimo]);

	if (!paso) return null;
	const sitio = sitioTarjeta(rect, alto);
	const idTitulo = "tutorial-titulo";

	return createPortal(
		<div className="fixed inset-0 z-[100]" data-tutorial-capa="">
			{rect ? (
				<div
					aria-hidden
					className="pointer-events-none absolute rounded-[14px] ring-2 ring-primary transition-all duration-200"
					style={{
						left: rect.x - AIRE,
						top: rect.y - AIRE,
						width: rect.w + 2 * AIRE,
						height: rect.h + 2 * AIRE,
						boxShadow: "0 0 0 9999px rgb(0 0 0 / 0.6)",
					}}
				/>
			) : (
				<div aria-hidden className="absolute inset-0 bg-black/60" />
			)}
			<div
				ref={tarjeta}
				role="dialog"
				aria-modal="true"
				aria-labelledby={idTitulo}
				data-slot="tutorial-tarjeta"
				className="absolute flex flex-col gap-2.5 rounded-[14px] border bg-card p-3.5 text-xs text-card-foreground shadow-xl transition-[left,top] duration-200"
				style={{ width: ANCHO, left: sitio.left, top: sitio.top }}
			>
				<div className="flex items-start gap-2">
					<h2 id={idTitulo} className="flex-1 text-sm font-semibold">
						{t(paso.titulo)}
					</h2>
					<Button
						size="icon-xs"
						variant="ghost"
						onClick={cerrar}
						aria-label={t("Cerrar el tutorial")}
					>
						<X />
					</Button>
				</div>
				<p className="leading-relaxed text-muted-foreground">{t(paso.texto)}</p>
				<div className="flex items-center gap-1.5">
					<span className="flex-1 text-[11px] text-muted-foreground">
						{t("Paso {a} de {b}", { a: i + 1, b: pasos.length })}
					</span>
					{i > 0 && (
						<Button size="sm" variant="ghost" onClick={() => setI(i - 1)}>
							<ChevronLeft />
							{t("Anterior")}
						</Button>
					)}
					<Button ref={siguiente} size="sm" onClick={() => (ultimo ? cerrar() : setI(i + 1))}>
						{ultimo ? t("Terminar") : t("Siguiente")}
						{!ultimo && <ChevronRight />}
					</Button>
				</div>
			</div>
		</div>,
		document.body,
	);
}

/**
 * Abre solo el tutorial de cada pantalla la primera vez que se entra en
 * ella, con un momento de espera para que esté pintada y si no hay un
 * diálogo abierto encima.
 */
export function useTutorialInicial() {
	const vista = useUi((s) => s.vista);
	useEffect(() => {
		const ui = useUi.getState();
		if (ui.tutorialesVistos.includes(vista) || ui.tutorial) return;
		const id = setTimeout(() => {
			const ahora = useUi.getState();
			if (ahora.vista !== vista || ahora.tutorial || ahora.dialogo) return;
			if (document.querySelector('[role="dialog"],[role="alertdialog"]')) return;
			ahora.abrirTutorial(vista);
		}, 700);
		return () => clearTimeout(id);
	}, [vista]);
}
