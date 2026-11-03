import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useFormato } from "@/lib/formato";
import { cn } from "@/lib/utils";
import { useUi, type Vista } from "@/stores/ui-store";
import { ANCHO, aplicarFormato, ladoMovil, type Rect, sitioTarjeta } from "./logica";
import { PASOS } from "./pasos";

/** Lo que el foco sobresale de la zona (px). */
const AIRE = 6;

/** El rectángulo de la zona en pantalla, si está y se ve. */
function zona(ancla: string | undefined): Rect | null {
	if (!ancla) return null;
	const el = document.querySelector(`[data-tutorial="${ancla}"]`);
	if (!el) return null;
	const r = el.getBoundingClientRect();
	if (r.width < 4 || r.height < 4) return null;
	return { x: r.left, y: r.top, w: r.width, h: r.height };
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
	const { tamano, tactil } = useFormato();
	const movil = tamano === "movil";
	const pasos = useMemo(() => aplicarFormato(PASOS[vista], tactil), [vista, tactil]);
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
		const el = paso?.ancla ? document.querySelector(`[data-tutorial="${paso.ancla}"]`) : null;
		// En tableta y móvil la zona puede estar fuera de la pantalla (dentro de
		// un panel que se desplaza): se trae a la vista antes de medirla.
		if (tactil) el?.scrollIntoView({ block: "center", inline: "nearest" });
		medir();
		const ro = new ResizeObserver(medir);
		if (el) ro.observe(el);
		window.addEventListener("resize", medir);
		return () => {
			ro.disconnect();
			window.removeEventListener("resize", medir);
		};
	}, [paso?.ancla, tamano]);

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
	const sitio = sitioTarjeta(rect, alto, window.innerWidth, window.innerHeight);
	const lado = ladoMovil(rect, window.innerHeight);
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
				className={cn(
					"absolute flex flex-col gap-2.5 rounded-[14px] border bg-card p-3.5 text-xs text-card-foreground shadow-xl",
					!movil && "transition-[left,top] duration-200",
				)}
				style={
					movil
						? {
								// En móvil: todo el ancho, pegada al lado contrario de la zona y
								// por encima de las zonas seguras.
								left: 16,
								right: 16,
								...(lado === "abajo"
									? { bottom: "calc(env(safe-area-inset-bottom) + 16px)" }
									: { top: "calc(env(safe-area-inset-top) + 16px)" }),
							}
						: { width: ANCHO, left: sitio.left, top: sitio.top }
				}
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
						<Icono icon="lucide:x" />
					</Button>
				</div>
				{movil ? (
					<ScrollArea viewportClassName="max-h-[36dvh]">
						<p className="leading-relaxed text-muted-foreground">{t(paso.texto)}</p>
					</ScrollArea>
				) : (
					<p className="leading-relaxed text-muted-foreground">{t(paso.texto)}</p>
				)}
				<div className="flex items-center gap-1.5">
					<span className="flex-1 text-[11px] text-muted-foreground">
						{t("Paso {a} de {b}", { a: i + 1, b: pasos.length })}
					</span>
					{i > 0 && (
						<Button size="sm" variant="ghost" onClick={() => setI(i - 1)}>
							<Icono icon="lucide:chevron-left" />
							{t("Anterior")}
						</Button>
					)}
					<Button ref={siguiente} size="sm" onClick={() => (ultimo ? cerrar() : setI(i + 1))}>
						{ultimo ? t("Terminar") : t("Siguiente")}
						{!ultimo && <Icono icon="lucide:chevron-right" />}
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
