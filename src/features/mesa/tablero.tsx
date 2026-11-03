import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
import type { Pos } from "@/bindings/Pos";
import type { Tapete as TapeteGuardado } from "@/bindings/Tapete";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { Zona } from "@/bindings/Zona";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useFormato } from "@/lib/formato";
import { urlArte, urlMesa } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { crearDetector, MS_PULSACION, TOLERANCIA_PULSACION } from "@/lib/pulsacion-larga";
import { cn } from "@/lib/utils";
import { cartasDe, cuantasEn, NINGUNA, tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { MS_SOLTADA, type Origen, origenDe, soltadaSigue, useVivo } from "@/stores/vivo-store";
import { colocar, destino, hacer, mover, sitio, zonaComun, zonaDe } from "./acciones";
import { destinoDeSoltar, pasaUmbral } from "./arrastre";
import { cartaDeArribaRevelada } from "./arriba-biblioteca";
import { atajoDe, teclaDe } from "./atajos";
import { type AlPulsar, anchoDeCarta, CartaMesa } from "./carta-mesa";
import { COLORES } from "./colores";
import { abanico, type Disposicion, disponer, posAPx, pxAPos, type Rect } from "./disposicion";
import * as vivo from "./en-vivo/emitir";
import { FantasmasVivos } from "./en-vivo/fantasmas";
import { Flechas } from "./flechas";
import { marcarArrastradas } from "./flip";
import { useMesaMovil } from "./formato-mesa";
import {
	conVista,
	crearDobleToque,
	esVistaEntera,
	limitarVista,
	type Punto,
	transformEntre,
	VISTA_ENTERA,
	type VistaCampo,
	vistaDeDosDedos,
} from "./gestos";
import { caraVisible, DorsoGenerico, ImagenCarta } from "./imagen-carta";
import { hayEncima, useLeerEnGrande } from "./leer-en-grande";
import { MenuBiblioteca, MenuCampo, MenuManoAjena, MenuZona } from "./menus/menu-zonas";
import { NotasSueltas } from "./notas-mesa";
import { elegirCarta } from "./objetivo";
import * as op from "./operaciones";
import { usePantalla } from "./pantalla";
import { campoAjeno, manoOcultaPorTurno } from "./privacidad";
import { acompanantes, conLasAjenas, puestosDelCampo } from "./puestos-campo";
import { VidaFantasma } from "./vida-fantasma";

/** Cuánto se mantiene pulsada una carta con el ratón para abrir su menú (si está activado), en ms. */
const MS_SOSTENIDO = 500;
/** Las zonas que se abren con un doble clic (o doble toque). */
const ZONAS_QUE_SE_ABREN: Zona[] = ["cementerio", "exilio", "mando", "banquillo"];
/** Sin cartas conocidas en una zona oculta (una referencia estable para el store). */
const SIN_VISIBLES: [number, IdCarta][] = [];

/**
 * **El tablero del jugador** (`capturas/pablo/00.jpg`): el tapete de fondo,
 * el campo libre, la mano en abanico abajo y las zonas a la derecha. Lleva
 * también el arrastre (eventos de puntero propios: la mesa es posición
 * libre con animación entre zonas, no listas ordenadas), el recuadro de
 * selección y los atajos de teclado.
 *
 * **Con el dedo** (plan del móvil, M3.9): un gesto por elemento. Sobre una
 * carta, moverse más de 8 px la arrastra, mantenerla quieta 500 ms abre su
 * menú, un toque la lee en grande y dos toques son el doble clic. En un hueco
 * del campo, un dedo hace el recuadro. Dos dedos mueven el campo y lo
 * acercan (`gestos.ts`): mientras se mueven solo se transforma la capa del
 * campo y al soltar se recoloca todo con el zoom nuevo.
 */
export function Tablero({ deckId }: { deckId: string | null }) {
	const { t } = useLang();
	const caja = useRef<HTMLDivElement>(null);
	const [tam, setTam] = useState({ w: 0, h: 0 });
	const yo = useMesa(tableroDe);
	const tamano = useUi((s) => s.mesa.tamanoCartas);
	const hayBanquillo = useMesa((s) => cuantasEn(s, zonaDe(yo, "banquillo")) > 0);
	const tapete = useMesa((s) => s.jugadores[yo]?.tapete ?? null);
	const modo = usePantalla((s) => s.modo);
	const { tactil } = useFormato();
	const movil = useMesaMovil();
	const [recuadro, setRecuadro] = useState<Rect | null>(null);
	// El zoom y el corrimiento del campo (solo con dos dedos), y si hay un gesto en marcha.
	const [vista, setVista] = useState<VistaCampo>(VISTA_ENTERA);
	const [gestoVivo, setGestoVivo] = useState(false);
	const capaCampo = useRef<HTMLDivElement>(null);
	// Los dedos que tocan ahora (por id de puntero) y lo que cancela el gesto de uno solo.
	const dedos = useRef(new Map<number, Punto>());
	const cancelarGesto = useRef<(() => void) | null>(null);
	const dobleToque = useRef(crearDobleToque());

	// Medido antes del primer pintado: sin él, el primer fotograma reparte las
	// cartas en un tablero de 0×0 y todas entran deslizándose desde la esquina.
	useLayoutEffect(() => {
		const el = caja.current;
		if (!el) return;
		const r = el.getBoundingClientRect();
		setTam({ w: r.width, h: r.height });
		let quieto = 0;
		const ro = new ResizeObserver(([e]) => {
			const p = usePantalla.getState();
			if (!p.redimensionando) p.setRedimensionando(true);
			window.clearTimeout(quieto);
			quieto = window.setTimeout(() => usePantalla.getState().setRedimensionando(false), 200);
			setTam({ w: e.contentRect.width, h: e.contentRect.height });
		});
		ro.observe(el);
		return () => {
			ro.disconnect();
			window.clearTimeout(quieto);
		};
	}, []);

	const base = useMemo(
		() => disponer(tam.w, tam.h, tamano, hayBanquillo, movil),
		[tam.w, tam.h, tamano, hayBanquillo, movil],
	);
	// Con zoom, `campo` es el campo ampliado y `visible` lo que se ve (`gestos.ts`).
	const disp = useMemo(() => conVista(base, vista), [base, vista]);
	const dispRef = useRef(disp);
	dispRef.current = disp;
	const baseRef = useRef(base);
	baseRef.current = base;
	const vistaRef = useRef(vista);
	vistaRef.current = vista;

	// Otro tablero, otro campo: se vuelve a verlo entero. Y si cambia de tamaño, no deja huecos.
	useEffect(() => setVista(VISTA_ENTERA), [yo]);
	useEffect(() => {
		setVista((v) => (esVistaEntera(v) ? v : limitarVista(v, base.campo.w, base.campo.h)));
	}, [base.campo.w, base.campo.h]);
	// La vista nueva ya está pintada: fuera el transform provisional del gesto.
	useLayoutEffect(() => {
		if (capaCampo.current) capaCampo.current.style.transform = "";
	}, [vista]);

	/** El punto del puntero relativo al tablero. */
	const punto = useCallback((e: { clientX: number; clientY: number }) => {
		const r = caja.current?.getBoundingClientRect();
		return { x: e.clientX - (r?.left ?? 0), y: e.clientY - (r?.top ?? 0) };
	}, []);

	// ── Dos dedos: mover y acercar el campo ──
	useEffect(() => {
		const el = caja.current;
		if (!el) return;
		let gesto: {
			vista: VistaCampo;
			ids: [number, number];
			a: Punto;
			b: Punto;
			nueva: VistaCampo;
		} | null = null;
		const alBajar = (e: PointerEvent) => {
			if (e.pointerType !== "touch") return;
			const r = el.getBoundingClientRect();
			dedos.current.set(e.pointerId, { x: e.clientX - r.left, y: e.clientY - r.top });
			if (dedos.current.size !== 2 || gesto) return;
			// El segundo dedo anula lo que hiciera el primero sobre una carta o el campo.
			cancelarGesto.current?.();
			const [[ia, a], [ib, b]] = [...dedos.current];
			if (ia === undefined || ib === undefined || !a || !b) return;
			gesto = { vista: vistaRef.current, ids: [ia, ib], a, b, nueva: vistaRef.current };
			setGestoVivo(true);
		};
		const alMover = (e: PointerEvent) => {
			if (!dedos.current.has(e.pointerId)) return;
			const r = el.getBoundingClientRect();
			dedos.current.set(e.pointerId, { x: e.clientX - r.left, y: e.clientY - r.top });
			if (!gesto) return;
			const a = dedos.current.get(gesto.ids[0]);
			const b = dedos.current.get(gesto.ids[1]);
			if (!a || !b) return;
			const c = baseRef.current.campo;
			gesto.nueva = vistaDeDosDedos(
				{ vista: gesto.vista, a: gesto.a, b: gesto.b },
				{ a, b },
				c.w,
				c.h,
			);
			if (capaCampo.current) {
				capaCampo.current.style.transform = transformEntre(gesto.vista, gesto.nueva);
			}
		};
		const alSoltar = (e: PointerEvent) => {
			if (!dedos.current.delete(e.pointerId)) return;
			if (!gesto?.ids.includes(e.pointerId)) return;
			const nueva = gesto.nueva;
			gesto = null;
			// Las cartas se recolocan de golpe en su sitio nuevo, sin deslizarse.
			const p = usePantalla.getState();
			p.setRedimensionando(true);
			window.setTimeout(() => usePantalla.getState().setRedimensionando(false), 200);
			setVista(nueva);
			setGestoVivo(false);
		};
		el.addEventListener("pointerdown", alBajar, true);
		window.addEventListener("pointermove", alMover);
		window.addEventListener("pointerup", alSoltar);
		window.addEventListener("pointercancel", alSoltar);
		return () => {
			el.removeEventListener("pointerdown", alBajar, true);
			window.removeEventListener("pointermove", alMover);
			window.removeEventListener("pointerup", alSoltar);
			window.removeEventListener("pointercancel", alSoltar);
		};
	}, []);

	const alDobleClic = useCallback((c: VistaCarta) => {
		if (campoAjeno(useMesa.getState(), c)) return;
		if (c.zona.tipo === "de" && c.zona.zona === "campo") void op.girar(c.id);
		else if (c.zona.tipo === "de" && c.zona.zona === "mano") {
			void mover([c.id], destino("campo", sitio.arriba, c.dueno));
		} else if (c.zona.tipo === "de" && ZONAS_QUE_SE_ABREN.includes(c.zona.zona)) {
			// Con el dedo no hay doble clic en la zona: dos toques en su carta de arriba la abren.
			usePantalla.getState().abrir({ tipo: "zona", jugador: c.zona.jugador, zona: c.zona.zona });
		}
	}, []);

	// ── Pulsar y arrastrar una carta ──
	const alPulsar: AlPulsar = useCallback(
		(e, c) => {
			if (e.button !== 0) return;
			e.stopPropagation();
			// Con dos dedos no se toca ninguna carta: es mover el campo.
			if (e.pointerType === "touch" && dedos.current.size > 1) return;
			const m = useMesa.getState();
			// Alt + clic: un aro de aviso sobre la carta, en todas las pantallas (en red).
			if (e.altKey) {
				vivo.senalar(c.id, null);
				return;
			}
			// Elegir objetivo (adjuntar, flecha).
			if (elegirCarta(c, t)) return;
			if (e.ctrlKey || e.metaKey || e.shiftKey) {
				m.alternar(c.id);
				return;
			}
			// La que se pulsa va la primera: es la que se lleva el puntero.
			const ids = m.seleccion.includes(c.id)
				? [c.id, ...m.seleccion.filter((x) => x !== c.id)]
				: [c.id];
			if (!m.seleccion.includes(c.id)) m.seleccionar([c.id]);
			const enCampo = c.zona.tipo === "de" && c.zona.zona === "campo";
			const p0 = punto(e);
			const tipo = e.pointerType;
			// El lápiz tiene «encima» y doble clic nativo (`carta-mesa.tsx`): va como el ratón.
			const conDedo = !hayEncima(tipo);
			let empezado = false;
			// Lo adjunto a lo que se lleva (equipos, auras, montones) se lleva con ello.
			let con: IdCarta[] = [];
			const elemento = e.currentTarget;
			const abrirMenu = (x: number, y: number) =>
				elemento.dispatchEvent(
					new MouseEvent("contextmenu", {
						bubbles: true,
						cancelable: true,
						clientX: x,
						clientY: y,
					}),
				);
			// Con el dedo: quieto 500 ms (sin moverse más de 8 px) abre el menú.
			let largo = false;
			let ultimo = p0;
			const detector = conDedo
				? crearDetector(
						(x, y) => {
							if (empezado) return;
							largo = true;
							abrirMenu(x, y);
						},
						MS_PULSACION,
						TOLERANCIA_PULSACION,
					)
				: null;
			detector?.empezar(e.clientX, e.clientY);
			// Con el ratón, mantener pulsado (Ajustes > Mesa) abre el menú de la carta.
			let sostenido: number | undefined;
			if (!conDedo && useUi.getState().mesa.clicSostenido) {
				const { clientX, clientY } = e;
				sostenido = window.setTimeout(() => {
					window.removeEventListener("pointermove", mueve);
					window.removeEventListener("pointerup", suelta);
					// Soltar el botón no elige la opción que quede debajo del menú.
					const tragar = (ev: Event) => {
						ev.stopPropagation();
						ev.preventDefault();
					};
					window.addEventListener("pointerup", tragar, { capture: true, once: true });
					window.addEventListener("click", tragar, { capture: true, once: true });
					abrirMenu(clientX, clientY);
				}, MS_SOSTENIDO);
			}
			const terminar = () => {
				detector?.cancelar();
				window.clearTimeout(sostenido);
				window.removeEventListener("pointermove", mueve);
				window.removeEventListener("pointerup", suelta);
				window.removeEventListener("pointercancel", anular);
				cancelarGesto.current = null;
			};
			// Un segundo dedo, o que el sistema se quede con el gesto: se deja todo como estaba.
			const anular = () => {
				terminar();
				if (!empezado) return;
				usePantalla.getState().setArrastre(null);
				vivo.cursor(
					tableroDe(useMesa.getState()),
					pxAPos(dispRef.current.campo, ultimo.x, ultimo.y),
					true,
				);
			};
			cancelarGesto.current = anular;
			const mueve = (ev: PointerEvent) => {
				detector?.mover(ev.clientX, ev.clientY);
				if (useRed.getState().papel === "espectador") return; // no se arrastra
				if (campoAjeno(useMesa.getState(), c)) return; // ni lo que no controlo
				const p = punto(ev);
				ultimo = p;
				if (!empezado && !pasaUmbral(p.x - p0.x, p.y - p0.y, tipo)) return;
				if (!empezado && enCampo) {
					const mesa = useMesa.getState();
					const delCampo = cartasDe(mesa, zonaDe(tableroDe(mesa), "campo"));
					const todas = conLasAjenas(delCampo, mesa.cartas);
					con = acompanantes(ids, puestosDelCampo(todas, mesa.cartas, dispRef.current).hijas);
				}
				empezado = true;
				detector?.cancelar();
				window.clearTimeout(sostenido);
				// En red, los demás ven la carta moverse (20 por segundo).
				vivo.arrastre(tableroDe(useMesa.getState()), ids, pxAPos(dispRef.current.campo, p.x, p.y));
				usePantalla.getState().setArrastre({
					ids,
					desde: enCampo
						? "campo"
						: c.zona.tipo === "de" && c.zona.zona === "mano"
							? "mano"
							: "zona",
					x: p.x,
					y: p.y,
					x0: p0.x,
					y0: p0.y,
					con,
				});
			};
			const suelta = (ev: PointerEvent) => {
				terminar();
				const llevaba = usePantalla.getState().arrastre;
				usePantalla.getState().setArrastre(null);
				if (!empezado) {
					// Un toque (no una pulsación larga, que ya abrió el menú): la lee en
					// grande y, dos seguidos, es el doble clic.
					if (conDedo && !largo) {
						useMesa.getState().setSobre(c.id);
						if (dobleToque.current.toque(c.id, performance.now(), ev.clientX, ev.clientY)) {
							alDobleClic(c);
						}
					}
					return;
				}
				// La carta deja de verse en vuelo; queda el cursor.
				const f = punto(ev);
				vivo.cursor(tableroDe(useMesa.getState()), pxAPos(dispRef.current.campo, f.x, f.y), true);
				// Dónde estaban, para no pintarlas ahí mientras llega el movimiento ni
				// animarlas otra vez desde allí: la carta se mueve y acaba donde toca.
				const antes = useMesa.getState().cartas;
				const origenes: Record<number, Origen> = {};
				for (const id of ids) {
					const carta = antes[id];
					if (carta) origenes[id] = origenDe(carta);
				}
				const destino = soltar(ids, f, p0, enCampo, dispRef.current, ev.shiftKey);
				if (destino) {
					marcarArrastradas([...ids, ...con]);
					useVivo.getState().soltar(origenes, performance.now());
					// Sigue pintada donde se soltó hasta que llega a su sitio: ni
					// desaparece un momento ni aparece en el de antes.
					if (llevaba) {
						usePantalla.getState().setArrastre({
							...llevaba,
							x: f.x,
							y: f.y,
							soltado: true,
							alCampo: destino.alCampo,
						});
					}
				}
			};
			window.addEventListener("pointermove", mueve);
			window.addEventListener("pointerup", suelta);
			window.addEventListener("pointercancel", anular);
		},
		[punto, t, alDobleClic],
	);

	// ── Recuadro de selección en el campo vacío ──
	const alPulsarCampo = (e: React.PointerEvent) => {
		if (e.button !== 0) return;
		if (e.pointerType === "touch" && dedos.current.size > 1) return;
		if (usePantalla.getState().modo) {
			usePantalla.getState().setModo(null);
			return;
		}
		const p0 = punto(e);
		const sumar = e.ctrlKey || e.metaKey || e.shiftKey;
		if (!sumar) useMesa.getState().seleccionar([]);
		const mueve = (ev: PointerEvent) => {
			const p = punto(ev);
			setRecuadro({
				x: Math.min(p0.x, p.x),
				y: Math.min(p0.y, p.y),
				w: Math.abs(p.x - p0.x),
				h: Math.abs(p.y - p0.y),
			});
		};
		const terminar = () => {
			window.removeEventListener("pointermove", mueve);
			window.removeEventListener("pointerup", suelta);
			window.removeEventListener("pointercancel", anular);
			cancelarGesto.current = null;
			setRecuadro(null);
		};
		// Un segundo dedo (es mover el campo) o el sistema se queda con el gesto.
		const anular = () => terminar();
		cancelarGesto.current = anular;
		const suelta = (ev: PointerEvent) => {
			terminar();
			const p = punto(ev);
			const r = {
				x: Math.min(p0.x, p.x),
				y: Math.min(p0.y, p.y),
				w: Math.abs(p.x - p0.x),
				h: Math.abs(p.y - p0.y),
			};
			if (r.w < 4 && r.h < 4) return;
			const m = useMesa.getState();
			const dentro = cartasDe(m, zonaDe(yo, "campo")).filter((id) => {
				const pos = m.cartas[id]?.pos;
				if (!pos) return false;
				const c = posAPx(dispRef.current.campo, pos);
				return c.x >= r.x && c.x <= r.x + r.w && c.y >= r.y && c.y <= r.y + r.h;
			});
			m.seleccionar(sumar ? [...new Set([...m.seleccion, ...dentro])] : dentro);
		};
		window.addEventListener("pointermove", mueve);
		window.addEventListener("pointerup", suelta);
		window.addEventListener("pointercancel", anular);
	};

	// ── El cursor, para que los demás lo vean (en red) ──
	useEffect(() => {
		const el = caja.current;
		if (!el) return;
		const alMover = (e: PointerEvent) => {
			if (usePantalla.getState().arrastre) return;
			const r = el.getBoundingClientRect();
			const campo = dispRef.current.campo;
			vivo.cursor(
				tableroDe(useMesa.getState()),
				pxAPos(campo, e.clientX - r.left, e.clientY - r.top),
			);
		};
		el.addEventListener("pointermove", alMover);
		return () => el.removeEventListener("pointermove", alMover);
	}, []);

	// ── Atajos de teclado ──
	useEffect(() => {
		const alTeclear = (e: KeyboardEvent) => {
			const el = e.target as HTMLElement | null;
			if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable)) {
				return;
			}
			if (e.altKey) return; // Alt+dígito son los avisos (`chat.tsx`)
			if (useRed.getState().papel === "espectador") return; // quien mira no juega
			if (usePantalla.getState().dialogo || document.querySelector('[role="dialog"]')) return;
			if (document.querySelector('[role="menu"]')) return;
			// Esc, además de soltar la selección, vuelve a ver el campo entero.
			if (e.key === "Escape") setVista(VISTA_ENTERA);
			const m = useMesa.getState();
			const carta = m.sobre ?? (m.seleccion.length > 0 ? (m.seleccion[0] ?? null) : null);
			const sobreLib = usePantalla.getState().zonaSobre === "biblioteca";
			const f = atajoDe(teclaDe(e), sobreLib ? null : carta, sobreLib);
			if (f) {
				e.preventDefault();
				f();
			}
		};
		window.addEventListener("keydown", alTeclear);
		return () => window.removeEventListener("keydown", alTeclear);
	}, []);

	const posMenu = useRef({ x: 5000, y: 5000 });
	const visible = disp.visible ?? disp.campo;
	// Con zoom (o moviéndolo) las cartas se recortan al hueco del campo.
	const recortar = gestoVivo || !esVistaEntera(vista);
	const recorte = recortar ? poligonoDe(visible) : undefined;

	return (
		<div
			ref={caja}
			className="relative size-full touch-none overflow-hidden select-none"
			data-tutorial="mesa-tablero"
		>
			{/* Hasta saber cuánto mide, nada que colocar (ni que deslizar desde 0×0). */}
			{tam.w > 0 && tam.h > 0 && (
				<>
					<Tapete tapete={tapete} deckId={deckId} />
					<MenuCampo jugador={yo} posicion={() => posMenu.current}>
						<div
							className="absolute"
							style={{
								left: visible.x,
								top: visible.y,
								width: visible.w,
								height: visible.h,
							}}
							onPointerDown={alPulsarCampo}
							// táctil: mantenerlo pulsado abre el menú (el sistema manda el mismo evento).
							onContextMenu={(e) => {
								const p = punto(e);
								posMenu.current = pxAPos(disp.campo, p.x, p.y);
							}}
							data-tutorial="mesa-campo"
						/>
					</MenuCampo>
					<Recortado recorte={recorte} z={5000}>
						<Bolitas campo={disp.campo} />
					</Recortado>
					{/* Las cartas del campo, en una capa sin tamaño que se recorta (con zoom) y
					    se transforma (mientras se mueve con dos dedos). Sin zoom no hace nada. */}
					<div
						className="absolute top-0 left-0 z-[1] size-0"
						style={recorte ? { clipPath: recorte } : undefined}
					>
						<div ref={capaCampo} className="absolute top-0 left-0 size-0 origin-top-left">
							<Campo disp={disp} alPulsar={alPulsar} alDobleClic={alDobleClic} />
						</div>
					</div>
					<Pila disp={base} alPulsar={alPulsar} />
					{!gestoVivo && (
						<Recortado recorte={recorte} z={6000}>
							<Flechas disp={disp} />
						</Recortado>
					)}
					<Mano disp={disp} alPulsar={alPulsar} alDobleClic={alDobleClic} />
					<Zonas disp={disp} alPulsar={alPulsar} />
					<VidaFantasma />
					<NotasSueltas />
					<Recortado recorte={recorte} z={8500}>
						<FantasmasVivos disp={disp} />
					</Recortado>
					<Arrastrado disp={disp} />
					{!esVistaEntera(vista) && !gestoVivo && (
						<button
							type="button"
							aria-label={t("Ver el campo entero")}
							onPointerDown={(e) => e.stopPropagation()}
							onClick={() => setVista(VISTA_ENTERA)}
							className="absolute z-[6700] flex size-11 items-center justify-center rounded-full border border-white/20 bg-black/70 text-white shadow-lg"
							style={{ left: visible.x + visible.w / 2 - 22, top: visible.y + visible.h - 52 }}
						>
							<Icono icon="lucide:minimize-2" className="size-5" />
						</button>
					)}
					{recuadro && (
						<div
							className="pointer-events-none absolute border border-primary bg-primary/15"
							style={{ left: recuadro.x, top: recuadro.y, width: recuadro.w, height: recuadro.h }}
						/>
					)}
					{modo && (
						<div className="pointer-events-none absolute top-14 left-1/2 -translate-x-1/2 rounded-full bg-primary px-3 py-1 text-xs text-primary-foreground shadow">
							{modo.tipo === "adjuntar"
								? tactil
									? t("Elige la carta a la que se adjunta (toca el campo vacío para cancelar)")
									: t("Elige la carta a la que se adjunta (Esc para cancelar)")
								: modo.tipo === "nota"
									? tactil
										? t(
												"Elige la carta del campo a la que se ata la nota (toca el campo vacío para cancelar)",
											)
										: t("Elige la carta del campo a la que se ata la nota (Esc para cancelar)")
									: tactil
										? t("Elige la carta o el jugador objetivo (toca el campo vacío para cancelar)")
										: t("Elige la carta o el jugador objetivo (Esc para cancelar)")}
						</div>
					)}
				</>
			)}
		</div>
	);
}

/** Lo que pasa al soltar lo arrastrado. */
function soltar(
	ids: IdCarta[],
	p: { x: number; y: number },
	p0: { x: number; y: number },
	desdeCampo: boolean,
	disp: Disposicion,
	alFondo: boolean,
): { alCampo: boolean } | null {
	const d = destinoDeSoltar(disp, p.x, p.y, alFondo);
	if (!d) return null;
	const m = useMesa.getState();
	const yo = tableroDe(m);
	const alCampo = d.zona.tipo === "de" && d.zona.zona === "campo";
	if (d.zona.tipo === "de" && d.zona.zona === "campo" && d.sitio.tipo === "enCampo") {
		if (desdeCampo) {
			// Cada una a su sitio, manteniendo las distancias, en un solo evento:
			// llegan juntas y un «deshacer» las devuelve juntas.
			const dx = p.x - p0.x;
			const dy = p.y - p0.y;
			const cartas: [IdCarta, Pos][] = [];
			for (const id of ids) {
				const pos = m.cartas[id]?.pos;
				if (!pos) continue;
				const c = posAPx(disp.campo, pos);
				cartas.push([id, pxAPos(disp.campo, c.x + dx, c.y + dy)]);
			}
			if (cartas.length === 1 && cartas[0])
				void mover([cartas[0][0]], destino("campo", sitio.campo(cartas[0][1]), yo));
			else if (cartas.length > 1) void colocar(cartas, yo);
			return { alCampo };
		}
		void mover(ids, { ...d, jugador: yo });
		return { alCampo };
	}
	// A la mano desde la mano: no se mueve (el orden de la mano es de pantalla).
	const desde = m.cartas[ids[0] ?? -1]?.zona;
	if (
		d.zona.tipo === "de" &&
		desde?.tipo === "de" &&
		desde.zona === d.zona.zona &&
		d.zona.zona !== "biblioteca"
	) {
		return null;
	}
	void mover(ids, d);
	return { alCampo };
}

/**
 * El tapete de fondo. El mío se lee de los ajustes (el del deck o el
 * general), con su velo y su desenfoque, y se relee al cambiarlo a media
 * partida; si no hay, el de la vista (el que se preparó al empezar).
 */
/** El tapete guardado de cada deck, leído una vez por sesión de la app. */
const TAPETES_LEIDOS: Record<string, TapeteGuardado | null> = {};
/** Los tapetes que ya se pintaron una vez: al volver, sin fundido. */
const TAPETES_PINTADOS = new Set<string>();

function Tapete({
	tapete,
	deckId,
}: {
	tapete: import("@/bindings/RefImagen").RefImagen | null;
	deckId: string | null;
}) {
	const version = usePantalla((s) => s.tapeteCambiado);
	// Lo último que se leyó de cada deck: al cambiar de tablero, el del deck
	// nuevo se ve ya (o, si nunca se leyó, ninguno) en vez de dejar un instante
	// el del deck anterior hasta que llegue el suyo. Sale de lo ya leído en esta
	// sesión (`TAPETES_LEIDOS`): volver a montar el tablero no lo hace parpadear.
	const [leidos, setLeidosEstado] = useState<Record<string, TapeteGuardado | null>>(() => ({
		...TAPETES_LEIDOS,
	}));
	const setLeidos = (
		f: (l: Record<string, TapeteGuardado | null>) => Record<string, TapeteGuardado | null>,
	) =>
		setLeidosEstado((l) => {
			const n = f(l);
			Object.assign(TAPETES_LEIDOS, n);
			return n;
		});
	// El que ya se pintó: el de ahora entra encima con un fundido, sin hueco.
	const [cargada, setCargada] = useState<string | null>(null);
	useEffect(() => {
		if (!deckId) return;
		let vivo = true;
		void (async () => {
			let t: TapeteGuardado | null = null;
			try {
				t =
					(await call("mesa_tapete_get", { deck: deckId })) ??
					(await call("mesa_tapete_get", { deck: null }));
			} catch {
				t = null;
			}
			if (vivo) setLeidos((l) => (l[deckId] === t ? l : { ...l, [deckId]: t }));
		})();
		return () => {
			vivo = false;
		};
	}, [deckId, version]);
	// Sin deck (el tablero de otro jugador) no se leen mis ajustes: vale el
	// tapete que la partida preparó para él.
	const sinLeer = deckId !== null && !(deckId in leidos);
	const propio = deckId ? (leidos[deckId] ?? null) : null;
	const url = sinLeer
		? null
		: propio
			? urlArte(propio.blob, 2000)
			: tapete
				? urlMesa(tapete, 1920)
				: null;
	const velo = propio ? propio.velo / 100 : 0.35;
	return (
		<div className="pointer-events-none absolute inset-0 overflow-hidden bg-gradient-to-b from-background to-muted/40">
			{url && (
				<img
					src={url}
					alt=""
					decoding="async"
					onLoad={() => {
						TAPETES_PINTADOS.add(url);
						setCargada(url);
					}}
					className={cn(
						"size-full object-cover transition-opacity duration-300",
						cargada === url || TAPETES_PINTADOS.has(url) ? "opacity-100" : "opacity-0",
					)}
					style={
						propio?.desenfoque
							? { filter: `blur(${propio.desenfoque}px)`, scale: "1.05" }
							: undefined
					}
					draggable={false}
				/>
			)}
			<div className="absolute inset-0 bg-black" style={{ opacity: velo }} />
		</div>
	);
}

/** Las cartas de mi campo, con sus adjuntas asomando y los montones juntos. */
function Campo({
	disp,
	alPulsar,
	alDobleClic,
}: {
	disp: Disposicion;
	alPulsar: AlPulsar;
	alDobleClic: (c: VistaCarta) => void;
}) {
	const yo = useMesa(tableroDe);
	const ids = useMesa((s) => cartasDe(s, zonaDe(yo, "campo")));
	const cartas = useMesa((s) => s.cartas);
	const { w, h } = disp.carta;
	// Las adjuntas y las amontonadas, junto a su carta (también las que cuelgan
	// de ella desde el campo de otro).
	const todas = conLasAjenas(ids, cartas);
	const { puestos } = puestosDelCampo(todas, cartas, disp);
	return (
		<>
			{todas.map((id) => {
				const p = puestos.get(id);
				if (!p) return null;
				return (
					<CartaMesa
						key={id}
						id={id}
						x={p.x}
						y={p.y}
						w={w}
						h={h}
						z={p.z}
						alPulsar={alPulsar}
						alDobleClic={alDobleClic}
					/>
				);
			})}
		</>
	);
}

/** La pila, a la izquierda: la de arriba, la primera de la lista. */
function Pila({ disp, alPulsar }: { disp: Disposicion; alPulsar: AlPulsar }) {
	const { t } = useLang();
	const movil = useMesaMovil();
	const ids = useMesa((s) => cartasDe(s, zonaComun("pila")));
	if (ids.length === 0) return null;
	const { w, h } = disp.cartaPequena;
	const arriba = [...ids].reverse();
	return (
		<div
			className={cn(
				"absolute top-14 left-2 rounded-lg border border-white/10 bg-black/50 p-2 backdrop-blur-sm",
				// En el móvil, a la derecha de la barra de jugadores.
				movil && "left-[7.5rem]",
			)}
			style={{
				width: w + 16,
				height: Math.min(disp.campo.h - 70, 24 + arriba.length * (h * 0.45) + h * 0.55),
			}}
			data-tutorial="mesa-pila"
		>
			<div className="mb-1 text-[10px] font-semibold tracking-wider text-white/80 uppercase">
				{t("Pila")}
			</div>
			{arriba.map((id, i) => (
				<CartaMesa
					key={id}
					id={id}
					x={8 + w / 2}
					y={24 + h / 2 + i * h * 0.45}
					w={w}
					h={h}
					z={100 - i}
					alPulsar={alPulsar}
					insignias={false}
				/>
			))}
		</div>
	);
}

/** La mano en abanico. */
function Mano({
	disp,
	alPulsar,
	alDobleClic,
}: {
	disp: Disposicion;
	alPulsar: AlPulsar;
	alDobleClic: (c: VistaCarta) => void;
}) {
	const { t } = useLang();
	const yo = useMesa(tableroDe);
	const todas = useMesa((s) => cartasDe(s, zonaDe(yo, "mano")));
	// En una partida local, la mano de quien no tiene el turno no se enseña.
	const enRed = useRed((s) => s.papel !== null);
	const mostrada = usePantalla((s) => s.manoMostrada);
	const privada = useMesa((s) => manoOcultaPorTurno(s, yo, enRed, mostrada));
	const nombre = useMesa((s) => s.jugadores[yo]?.nombre ?? "");
	const ids = privada ? NINGUNA : todas;
	// La mano de otro (en red): solo cuántas, boca abajo.
	const ocultas = useMesa((s) =>
		s.zonas[`${yo}:mano`]?.cartas == null ? cuantasEn(s, zonaDe(yo, "mano")) : 0,
	);
	const ocultasTotal = ocultas + (privada ? todas.length : 0);
	const dorso = useMesa((s) => s.jugadores[yo]?.dorso ?? null);
	// De la mano de otro, las que conozco (mirada, enseñadas) van boca arriba en su sitio.
	const visibles = useMesa((s) => s.zonas[`${yo}:mano`]?.visibles ?? SIN_VISIBLES);
	const ajena = useMesa((s) => s.yo !== null && s.yo !== yo);
	const conocidas = new Map(ocultas > 0 && !privada ? visibles : []);
	const { w, h } = disp.cartaPequena;
	const n = ids.length + ocultasTotal;
	const pos = abanico(disp.mano, n, { w, h });
	return (
		<>
			<div
				className="pointer-events-none absolute text-center text-[11px] text-white/80"
				style={{ left: disp.mano.x, width: disp.mano.w, top: disp.mano.y - 14 }}
				data-tutorial="mesa-mano"
			>
				{n === 1 ? t("Una carta en la mano") : t("{n} cartas en la mano", { n })}
			</div>
			{privada && (
				// La cortina: quien responde fuera de su turno destapa su mano.
				<button
					type="button"
					className="absolute flex items-center justify-center gap-1.5 rounded-md border border-dashed border-white/40 bg-black/55 text-xs font-medium text-white backdrop-blur-sm hover:bg-black/70"
					style={{
						left: disp.mano.x,
						top: disp.mano.y,
						width: disp.mano.w,
						height: disp.mano.h,
						zIndex: 5000,
					}}
					onClick={() => usePantalla.getState().mostrarMano(yo)}
				>
					<Icono icon="lucide:eye" className="size-4" />
					{t("Mano de {nombre}: pulsa para verla", { nombre })}
				</button>
			)}
			{ajena && (
				<MenuManoAjena jugador={yo}>
					<div
						className="absolute"
						style={{ left: disp.mano.x, top: disp.mano.y, width: disp.mano.w, height: disp.mano.h }}
					/>
				</MenuManoAjena>
			)}
			{[...conocidas].map(([i, id]) => {
				const p = pos[i];
				if (!p) return null;
				return (
					<CartaMesa
						key={id}
						id={id}
						x={p.x}
						y={p.y}
						w={w}
						h={h}
						giro={p.giro}
						z={1000 + i}
						alPulsar={alPulsar}
						alDobleClic={alDobleClic}
						insignias={false}
						levantar
					/>
				);
			})}
			{Array.from({ length: ocultasTotal }, (_, i) => {
				const p = pos[i];
				if (!p || conocidas.has(i)) return null;
				return (
					<div
						key={i}
						className="pointer-events-none absolute"
						style={{
							left: p.x - w / 2,
							top: p.y - h / 2,
							width: w,
							height: h,
							transform: `rotate(${p.giro}deg)`,
							zIndex: 1000 + i,
						}}
					>
						{dorso ? <ImagenCarta imagen={dorso} ancho={244} /> : <DorsoGenerico />}
					</div>
				);
			})}
			{ids.map((id, j) => {
				const i = ocultasTotal + j;
				const p = pos[i];
				if (!p) return null;
				return (
					<CartaMesa
						key={id}
						id={id}
						x={p.x}
						y={p.y}
						w={w}
						h={h}
						giro={p.giro}
						z={1000 + i}
						alPulsar={alPulsar}
						alDobleClic={alDobleClic}
						insignias={false}
						levantar
					/>
				);
			})}
		</>
	);
}

/** Biblioteca, cementerio, exilio, mando y banquillo. */
function Zonas({ disp, alPulsar }: { disp: Disposicion; alPulsar: AlPulsar }) {
	const { t } = useLang();
	const yo = useMesa(tableroDe);
	const lib = useMesa((s) => s.zonas[`${yo}:biblioteca`]);
	const dorso = useMesa((s) => s.jugadores[yo]?.dorso ?? null);
	// La de arriba se pinta boca arriba si se juega así o si se acaba de revelar;
	// mirarla (escrutar, mirar) no la pone boca arriba en la mesa.
	const cartaArriba = useMesa((s) => cartaDeArribaRevelada(s, yo));
	const arribaLib = cartaArriba?.id ?? null;
	// Revelada a todos (no por jugar así): se puede dejar de revelar con el botón.
	const arribaRevelada = cartaArriba?.revelada ?? false;
	const z = disp.zonas;
	// Leer en grande la de arriba revelada: con el ratón, al pasar; con el dedo, un toque
	// en su botón de la lupa (tocar la biblioteca roba).
	const leerArriba = useLeerEnGrande(arribaLib, false);
	const { tactil } = useFormato();
	// Con las zonas encogidas del móvil de pie el nombre no cabe: letra menor y puntos.
	const zona = (r: Rect, nombre: string, hijo: React.ReactNode, n: number, alVer?: () => void) => {
		const estrecha = r.w < 41;
		return (
			<div
				className="absolute flex flex-col items-center"
				style={{ left: r.x, top: r.y - 16, width: r.w, height: r.h + 16 }}
			>
				{alVer && tactil ? (
					// Con el dedo no hay doble clic: tocar el nombre de la zona la abre (con el
					// ratón, como siempre, el doble clic en la zona).
					<button
						type="button"
						onClick={alVer}
						className={cn(
							"relative h-4 text-[10px] text-white/80 underline decoration-dotted before:absolute before:-inset-x-1 before:-inset-y-3 before:content-['']",
							estrecha && "max-w-full truncate text-[9px]",
						)}
					>
						{nombre}
						{n > 0 ? ` · ${n}` : ""}
					</button>
				) : (
					<span
						className={cn(
							"h-4 text-[10px] text-white/80",
							estrecha && "max-w-full truncate text-[9px]",
						)}
					>
						{nombre}
						{n > 0 ? ` · ${n}` : ""}
					</span>
				)}
				<div className="relative w-full flex-1 rounded-[5%] border border-dashed border-white/25 bg-black/20">
					{hijo}
				</div>
			</div>
		);
	};
	return (
		<>
			<MenuBiblioteca jugador={yo}>
				<div
					// táctil: la zona bajo el ratón es solo de los atajos de teclado; la carta de
					// arriba revelada se lee con la lupa de abajo.
					onPointerEnter={(e) => {
						usePantalla.getState().setZonaSobre("biblioteca");
						// La revelada se lee en grande, como cualquier carta de la mesa.
						leerArriba?.onPointerEnter(e);
					}}
					// táctil: ídem.
					onPointerLeave={(e) => {
						usePantalla.getState().setZonaSobre(null);
						leerArriba?.onPointerLeave(e);
					}}
					// Pulsar la propia roba; la de otro no hace nada sin querer (va por el menú).
					onClick={() => yo === useMesa.getState().yo && void op.robar()}
					className="cursor-pointer"
					data-tutorial="mesa-biblioteca"
				>
					{zona(
						z.biblioteca,
						t("Biblioteca"),
						(lib?.cuantas ?? 0) > 0 &&
							(arribaLib !== null ? (
								<>
									<CartaSuelta id={arribaLib} />
									<button
										type="button"
										aria-label={t("Leer la de arriba en grande")}
										onPointerDown={(e) => e.stopPropagation()}
										onClick={(e) => {
											e.stopPropagation();
											useMesa.getState().setSobre(arribaLib);
										}}
										data-lee=""
										className="absolute top-1 left-1 hidden size-6 items-center justify-center rounded-full bg-black/75 text-white ring-1 ring-white/40 before:absolute before:-inset-2 before:content-[''] [[data-tactil]_&]:flex"
									>
										<Icono icon="lucide:zoom-in" className="size-3.5" />
									</button>
									{arribaRevelada && (
										<WithTooltip content={t("Dejar de revelar la de arriba")}>
											<button
												type="button"
												aria-label={t("Dejar de revelar la de arriba")}
												onPointerDown={(e) => e.stopPropagation()}
												onClick={(e) => {
													e.stopPropagation();
													void op.hacerPor(yo, { que: "ocultarArriba", n: 1 });
												}}
												className="absolute top-1 right-1 flex size-6 items-center justify-center rounded-full bg-black/75 text-white ring-1 ring-white/40 hover:bg-primary"
											>
												<Icono icon="lucide:eye-off" className="size-3.5" />
											</button>
										</WithTooltip>
									)}
								</>
							) : dorso ? (
								<ImagenCarta imagen={dorso} ancho={244} />
							) : (
								<DorsoGenerico />
							)),
						lib?.cuantas ?? 0,
					)}
				</div>
			</MenuBiblioteca>
			<ZonaArriba
				disp={disp}
				zona="cementerio"
				nombre={t("Cementerio")}
				r={z.cementerio}
				alPulsar={alPulsar}
				hacerZona={zona}
			/>
			<ZonaArriba
				disp={disp}
				zona="exilio"
				nombre={t("Exilio")}
				r={z.exilio}
				alPulsar={alPulsar}
				hacerZona={zona}
			/>
			<ZonaArriba
				disp={disp}
				zona="mando"
				nombre={t("Mando")}
				r={z.mando}
				alPulsar={alPulsar}
				hacerZona={zona}
			/>
			{z.banquillo.w > 0 && (
				<ZonaArriba
					disp={disp}
					zona="banquillo"
					nombre={t("Banquillo")}
					r={z.banquillo}
					alPulsar={alPulsar}
					hacerZona={zona}
				/>
			)}
		</>
	);
}

/** Una carta pintada sin más (la de arriba de la biblioteca revelada). */
function CartaSuelta({ id }: { id: IdCarta }) {
	const c = useMesa((s) => s.cartas[id]);
	if (!c) return null;
	return <ImagenCarta imagen={caraVisible(c)} ancho={244} nombre={c.origen?.nombre} />;
}

/** Cementerio, exilio y mando: la de arriba, que se puede arrastrar. */
function ZonaArriba({
	zona,
	nombre,
	r,
	alPulsar,
	hacerZona,
}: {
	disp: Disposicion;
	zona: "cementerio" | "exilio" | "mando" | "banquillo";
	nombre: string;
	r: Rect;
	alPulsar: AlPulsar;
	hacerZona: (
		r: Rect,
		nombre: string,
		hijo: React.ReactNode,
		n: number,
		alVer?: () => void,
	) => React.ReactNode;
}) {
	const yo = useMesa(tableroDe);
	const v = useMesa((s) => s.zonas[`${yo}:${zona}`]);
	const ids = v?.cartas ?? [];
	const arriba = ids[ids.length - 1];
	const ver = () => usePantalla.getState().abrir({ tipo: "zona", jugador: yo, zona });
	return (
		<MenuZona zona={zona} jugador={yo}>
			<div onDoubleClick={ver} data-tutorial={`mesa-${zona}`}>
				{/* Con el dedo no hay doble clic: tocar el nombre de la zona la abre. */}
				{hacerZona(
					r,
					nombre,
					zona === "banquillo"
						? (v?.cuantas ?? 0) > 0 && (
								<button
									type="button"
									onClick={ver}
									className="flex size-full items-center justify-center"
								>
									<Icono icon="lucide:layers" className="size-6 text-white/60" />
								</button>
							)
						: arriba !== undefined && (
								<CartaMesa
									id={arriba}
									x={r.w / 2}
									y={r.h / 2}
									w={r.w}
									h={r.h}
									z={1}
									alPulsar={alPulsar}
									insignias={false}
								/>
							),
					v?.cuantas ?? 0,
					ver,
				)}
			</div>
		</MenuZona>
	);
}

/** Las bolitas de colores del campo. */
/** El `clip-path` que deja ver solo `r`. */
function poligonoDe(r: Rect): string {
	return `polygon(${r.x}px ${r.y}px, ${r.x + r.w}px ${r.y}px, ${r.x + r.w}px ${r.y + r.h}px, ${r.x}px ${r.y + r.h}px)`;
}

/**
 * Lo que se pinta sobre el campo (bolitas, flechas, fantasmas) recortado, con
 * zoom, al campo que se ve, como las cartas: así no queda encima de la mano ni
 * de las zonas (ni recoge sus toques; `clip-path` también recorta dónde se
 * toca). Sin zoom no envuelve nada: todo queda como siempre.
 */
function Recortado({
	recorte,
	z,
	children,
}: {
	recorte: string | undefined;
	z: number;
	children: React.ReactNode;
}) {
	if (!recorte) return <>{children}</>;
	return (
		<div className="pointer-events-none absolute inset-0" style={{ clipPath: recorte, zIndex: z }}>
			{children}
		</div>
	);
}

function Bolitas({ campo }: { campo: Rect }) {
	const yo = useMesa(tableroDe);
	const bolitas = useMesa((s) => s.general?.bolitas ?? NINGUNA);
	const lado = { S: 12, M: 18, L: 26 };
	return (
		<>
			{bolitas
				.filter((b) => b.campoDe === yo)
				.map((b) => {
					const p = posAPx(campo, b.pos);
					const d = lado[b.tamano];
					return (
						<button
							key={b.id}
							type="button"
							onDoubleClick={() => void hacer({ que: "quitarBolita", id: b.id })}
							// Con zoom van en una capa recortada que no recoge toques (`Recortado`).
							className="pointer-events-auto absolute rounded-full shadow ring-2 ring-white/60"
							style={{
								left: p.x - d / 2,
								top: p.y - d / 2,
								width: d,
								height: d,
								background: COLORES[b.color] ?? "#fff",
								zIndex: 5000,
							}}
						/>
					);
				})}
		</>
	);
}

/**
 * Lo que se arrastra, siguiendo al puntero (sin esperar a Rust). Al soltar se
 * queda donde se soltó hasta que la carta llega a su sitio (`soltado`): así no
 * desaparece un momento ni se ve en el de antes.
 */
function Arrastrado({ disp }: { disp: Disposicion }) {
	const a = usePantalla((s) => s.arrastre);
	const cartas = useMesa((s) => s.cartas);
	const yo = useMesa(tableroDe);
	const delCampo = useMesa((s) => cartasDe(s, zonaDe(yo, "campo")));
	const calidad = useUi((s) => s.mesa.calidad);
	// Ya soltada, cuando todas han llegado se quita en el mismo fotograma en que
	// las reales (y sus adjuntas) ocupan su sitio; si el movimiento no llega, al
	// rato.
	useLayoutEffect(() => {
		if (!a?.soltado) return;
		const llegaron = a.ids.every((id) => {
			const c = cartas[id];
			const s = useVivo.getState().soltadas[id];
			return !c || !s || !soltadaSigue(s, c);
		});
		if (llegaron) {
			usePantalla.getState().setArrastre(null);
			return;
		}
		const t = window.setTimeout(() => usePantalla.getState().setArrastre(null), MS_SOLTADA);
		return () => window.clearTimeout(t);
	}, [a, cartas]);
	if (!a) return null;
	const dx = a.x - a.x0;
	const dy = a.y - a.y0;
	const { w, h } = a.desde === "campo" || a.alCampo ? disp.carta : disp.cartaPequena;
	const ancho = a.desde === "campo" || a.alCampo ? anchoDeCarta(w, calidad) : 244;
	// Lo adjunto a lo que se lleva va con ello, detrás, con su misma geometría del campo.
	const { puestos } = puestosDelCampo(
		a.con.length > 0 ? conLasAjenas(delCampo, cartas) : [],
		cartas,
		disp,
	);
	return (
		<div className="pointer-events-none absolute inset-0" style={{ zIndex: 9000 }}>
			{a.con.map((id) => {
				const c = cartas[id];
				const p = puestos.get(id);
				if (!c || !p) return null;
				// Soltada su carta y ya en su sitio: las reales ocupan el lugar de estas.
				if (a.soltado) {
					const madre = c.adjuntaA ?? c.agrupadaBajo;
					const mc = madre === null ? undefined : cartas[madre];
					const s = madre === null ? undefined : useVivo.getState().soltadas[madre];
					if (!mc || !s || !soltadaSigue(s, mc)) return null;
				}
				return (
					<div
						key={id}
						className={cn(
							"absolute top-0 left-0 will-change-transform",
							!a.soltado && "opacity-90 shadow-2xl",
						)}
						style={{
							width: w,
							height: h,
							transform: `translate(${p.x + dx - w / 2}px, ${p.y + dy - h / 2}px) rotate(${c.girada ? 90 : 0}deg)`,
						}}
					>
						<ImagenCarta imagen={caraVisible(c)} ancho={anchoDeCarta(w, calidad)} />
					</div>
				);
			})}
			{a.ids.map((id, i) => {
				const c = cartas[id];
				if (!c) return null;
				// Soltada y ya en su sitio: la real ocupa el lugar de esta, en el mismo fotograma.
				if (a.soltado) {
					const s = useVivo.getState().soltadas[id];
					if (!s || !soltadaSigue(s, c)) return null;
				}
				const centro =
					a.desde === "campo" && c.pos
						? posAPx(disp.campo, c.pos)
						: { x: a.x0 + i * 12, y: a.y0 + i * 6 };
				return (
					<div
						key={id}
						className={cn(
							"absolute top-0 left-0 will-change-transform",
							!a.soltado && "opacity-90 shadow-2xl",
						)}
						style={{
							width: w,
							height: h,
							transform: `translate(${centro.x + dx - w / 2}px, ${centro.y + dy - h / 2}px) rotate(${c.girada ? 90 : 0}deg)`,
						}}
					>
						<ImagenCarta imagen={caraVisible(c)} ancho={ancho} />
					</div>
				);
			})}
		</div>
	);
}
