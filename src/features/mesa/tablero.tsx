import { Eye, EyeOff, Layers } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { IdCarta } from "@/bindings/IdCarta";
import type { Tapete as TapeteGuardado } from "@/bindings/Tapete";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { useLang } from "@/components/language-provider";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { urlArte, urlMesa } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { cartasDe, cuantasEn, NINGUNA, tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { MS_SOLTADA, type Origen, origenDe, soltadaSigue, useVivo } from "@/stores/vivo-store";
import { destino, hacer, mover, sitio, zonaComun, zonaDe } from "./acciones";
import { destinoDeSoltar, pasaUmbral } from "./arrastre";
import { cartaDeArribaRevelada } from "./arriba-biblioteca";
import { atajoDe, teclaDe } from "./atajos";
import { type AlPulsar, CartaMesa } from "./carta-mesa";
import { COLORES } from "./colores";
import { abanico, type Disposicion, disponer, posAPx, pxAPos, type Rect } from "./disposicion";
import * as vivo from "./en-vivo/emitir";
import { FantasmasVivos } from "./en-vivo/fantasmas";
import { ataqueAUnCompanero } from "./equipos";
import { marcarArrastradas } from "./flip";
import { caraVisible, DorsoGenerico, ImagenCarta } from "./imagen-carta";
import { MenuBiblioteca, MenuCampo, MenuManoAjena, MenuZona } from "./menus/menu-zonas";
import * as op from "./operaciones";
import { usePantalla } from "./pantalla";
import { manoOcultaPorTurno } from "./privacidad";
import { acompanantes, conLasAjenas, puestosDelCampo } from "./puestos-campo";

/** Cuánto se mantiene pulsada una carta para abrir su menú (si está activado), en ms. */
const MS_SOSTENIDO = 500;
/** Sin cartas conocidas en una zona oculta (una referencia estable para el store). */
const SIN_VISIBLES: [number, IdCarta][] = [];

/**
 * **El tablero del jugador** (`capturas/pablo/00.jpg`): el tapete de fondo,
 * el campo libre, la mano en abanico abajo y las zonas a la derecha. Lleva
 * también el arrastre (eventos de puntero propios: la mesa es posición
 * libre con animación entre zonas, no listas ordenadas), el recuadro de
 * selección y los atajos de teclado.
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
	const [recuadro, setRecuadro] = useState<Rect | null>(null);

	useEffect(() => {
		const el = caja.current;
		if (!el) return;
		const ro = new ResizeObserver(([e]) => {
			setTam({ w: e.contentRect.width, h: e.contentRect.height });
		});
		ro.observe(el);
		return () => ro.disconnect();
	}, []);

	const disp = useMemo(
		() => disponer(tam.w, tam.h, tamano, hayBanquillo),
		[tam.w, tam.h, tamano, hayBanquillo],
	);
	const dispRef = useRef(disp);
	dispRef.current = disp;

	/** El punto del puntero relativo al tablero. */
	const punto = useCallback((e: { clientX: number; clientY: number }) => {
		const r = caja.current?.getBoundingClientRect();
		return { x: e.clientX - (r?.left ?? 0), y: e.clientY - (r?.top ?? 0) };
	}, []);

	// ── Pulsar y arrastrar una carta ──
	const alPulsar: AlPulsar = useCallback(
		(e, c) => {
			if (e.button !== 0) return;
			e.stopPropagation();
			const m = useMesa.getState();
			const pantalla = usePantalla.getState();
			// Alt + clic: un aro de aviso sobre la carta, en todas las pantallas (en red).
			if (e.altKey) {
				vivo.senalar(c.id, null);
				return;
			}
			// Elegir objetivo (adjuntar, flecha).
			if (pantalla.modo) {
				const md = pantalla.modo;
				pantalla.setModo(null);
				if (md.tipo === "adjuntar" && md.carta !== c.id) {
					void op.adjuntar(md.carta, c.id);
				} else if (md.tipo === "flecha" && md.de !== c.id) {
					const origen = m.cartas[md.de];
					if (
						md.flecha === "ataque" &&
						origen &&
						ataqueAUnCompanero(m.jugadores, m.variantes, origen.controlador, c.controlador)
					) {
						toast.warning(t("Ese jugador es de tu equipo y el fuego amigo está apagado."));
					}
					void hacer({
						que: "flecha",
						de: { tipo: "carta", id: md.de },
						a: { tipo: "carta", id: c.id },
						tipo: md.flecha,
					});
				}
				return;
			}
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
			let empezado = false;
			// Lo adjunto a lo que se lleva (equipos, auras, montones) se lleva con ello.
			let con: IdCarta[] = [];
			// Mantener pulsado (Ajustes > Mesa) abre el menú de la carta.
			let sostenido: number | undefined;
			if (useUi.getState().mesa.clicSostenido) {
				const elemento = e.currentTarget;
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
					elemento.dispatchEvent(
						new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX, clientY }),
					);
				}, MS_SOSTENIDO);
			}
			const mueve = (ev: PointerEvent) => {
				if (useRed.getState().papel === "espectador") return; // no se arrastra
				const p = punto(ev);
				if (!empezado && !pasaUmbral(p.x - p0.x, p.y - p0.y)) return;
				if (!empezado && enCampo) {
					const mesa = useMesa.getState();
					const delCampo = cartasDe(mesa, zonaDe(tableroDe(mesa), "campo"));
					const todas = conLasAjenas(delCampo, mesa.cartas);
					con = acompanantes(ids, puestosDelCampo(todas, mesa.cartas, dispRef.current).hijas);
				}
				empezado = true;
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
				window.clearTimeout(sostenido);
				window.removeEventListener("pointermove", mueve);
				window.removeEventListener("pointerup", suelta);
				const llevaba = usePantalla.getState().arrastre;
				usePantalla.getState().setArrastre(null);
				if (!empezado) return;
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
		},
		[punto],
	);

	const alDobleClic = useCallback((c: VistaCarta) => {
		if (c.zona.tipo === "de" && c.zona.zona === "campo") void op.girar(c.id);
		else if (c.zona.tipo === "de" && c.zona.zona === "mano") {
			void mover([c.id], destino("campo", sitio.arriba, c.dueno));
		}
	}, []);

	// ── Recuadro de selección en el campo vacío ──
	const alPulsarCampo = (e: React.PointerEvent) => {
		if (e.button !== 0) return;
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
		const suelta = (ev: PointerEvent) => {
			window.removeEventListener("pointermove", mueve);
			window.removeEventListener("pointerup", suelta);
			setRecuadro(null);
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

	return (
		<div
			ref={caja}
			className="relative size-full overflow-hidden select-none"
			data-tutorial="mesa-tablero"
		>
			<Tapete tapete={tapete} deckId={deckId} />
			<MenuCampo jugador={yo} posicion={() => posMenu.current}>
				<div
					className="absolute"
					style={{
						left: disp.campo.x,
						top: disp.campo.y,
						width: disp.campo.w,
						height: disp.campo.h,
					}}
					onPointerDown={alPulsarCampo}
					onContextMenu={(e) => {
						const p = punto(e);
						posMenu.current = pxAPos(disp.campo, p.x, p.y);
					}}
					data-tutorial="mesa-campo"
				/>
			</MenuCampo>
			<Bolitas campo={disp.campo} />
			<Campo disp={disp} alPulsar={alPulsar} alDobleClic={alDobleClic} />
			<Pila disp={disp} alPulsar={alPulsar} />
			<Flechas disp={disp} />
			<Mano disp={disp} alPulsar={alPulsar} alDobleClic={alDobleClic} />
			<Zonas disp={disp} alPulsar={alPulsar} />
			<Fantasma disp={disp} />
			<FantasmasVivos disp={disp} />
			<Arrastrado disp={disp} />
			{recuadro && (
				<div
					className="pointer-events-none absolute border border-primary bg-primary/15"
					style={{ left: recuadro.x, top: recuadro.y, width: recuadro.w, height: recuadro.h }}
				/>
			)}
			{modo && (
				<div className="pointer-events-none absolute top-14 left-1/2 -translate-x-1/2 rounded-full bg-primary px-3 py-1 text-xs text-primary-foreground shadow">
					{modo.tipo === "adjuntar"
						? t("Elige la carta a la que se adjunta (Esc para cancelar)")
						: t("Elige la carta o el jugador objetivo (Esc para cancelar)")}
				</div>
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
			// Cada una a su sitio, manteniendo las distancias.
			const dx = p.x - p0.x;
			const dy = p.y - p0.y;
			for (const id of ids) {
				const pos = m.cartas[id]?.pos;
				if (!pos) continue;
				const c = posAPx(disp.campo, pos);
				const nueva = pxAPos(disp.campo, c.x + dx, c.y + dy);
				void mover([id], destino("campo", sitio.campo(nueva), yo));
			}
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
function Tapete({
	tapete,
	deckId,
}: {
	tapete: import("@/bindings/RefImagen").RefImagen | null;
	deckId: string | null;
}) {
	const version = usePantalla((s) => s.tapeteCambiado);
	const [propio, setPropio] = useState<TapeteGuardado | null>(null);
	useEffect(() => {
		let vivo = true;
		void (async () => {
			try {
				// Sin deck (el tablero de otro jugador) no se leen mis ajustes: vale el
				// tapete que la partida preparó para él.
				const t = deckId
					? ((await call("mesa_tapete_get", { deck: deckId })) ??
						(await call("mesa_tapete_get", { deck: null })))
					: null;
				if (vivo) setPropio(t);
			} catch {
				if (vivo) setPropio(null);
			}
		})();
		return () => {
			vivo = false;
		};
	}, [deckId, version]);
	const url = propio ? urlArte(propio.blob, 2000) : tapete ? urlMesa(tapete, 1920) : null;
	const velo = propio ? propio.velo / 100 : 0.35;
	return (
		<div className="pointer-events-none absolute inset-0 overflow-hidden bg-gradient-to-b from-background to-muted/40">
			{url && (
				<img
					src={url}
					alt=""
					className="size-full object-cover"
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
	const ids = useMesa((s) => cartasDe(s, zonaComun("pila")));
	if (ids.length === 0) return null;
	const { w, h } = disp.cartaPequena;
	const arriba = [...ids].reverse();
	return (
		<div
			className="absolute top-14 left-2 rounded-lg border border-white/10 bg-black/50 p-2 backdrop-blur-sm"
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
	const privada = useMesa((s) => manoOcultaPorTurno(s, yo, enRed));
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
	const zona = (r: Rect, nombre: string, hijo: React.ReactNode, n: number) => (
		<div
			className="absolute flex flex-col items-center"
			style={{ left: r.x, top: r.y - 16, width: r.w, height: r.h + 16 }}
		>
			<span className="h-4 text-[10px] text-white/80">
				{nombre}
				{n > 0 ? ` · ${n}` : ""}
			</span>
			<div className="relative w-full flex-1 rounded-[5%] border border-dashed border-white/25 bg-black/20">
				{hijo}
			</div>
		</div>
	);
	return (
		<>
			<MenuBiblioteca jugador={yo}>
				<div
					onPointerEnter={() => {
						usePantalla.getState().setZonaSobre("biblioteca");
						// La revelada se lee en grande, como cualquier carta de la mesa.
						if (arribaLib !== null) useMesa.getState().setSobre(arribaLib);
					}}
					onPointerLeave={() => {
						usePantalla.getState().setZonaSobre(null);
						if (arribaLib !== null && useMesa.getState().sobre === arribaLib) {
							useMesa.getState().setSobre(null);
						}
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
												<EyeOff className="size-3.5" />
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
	hacerZona: (r: Rect, nombre: string, hijo: React.ReactNode, n: number) => React.ReactNode;
}) {
	const yo = useMesa(tableroDe);
	const v = useMesa((s) => s.zonas[`${yo}:${zona}`]);
	const ids = v?.cartas ?? [];
	const arriba = ids[ids.length - 1];
	const ver = () => usePantalla.getState().abrir({ tipo: "zona", jugador: yo, zona });
	return (
		<MenuZona zona={zona} jugador={yo}>
			<div onDoubleClick={ver} data-tutorial={`mesa-${zona}`}>
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
									<Layers className="size-6 text-white/60" />
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
				)}
			</div>
		</MenuZona>
	);
}

/** Las bolitas de colores del campo. */
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
							className="absolute rounded-full shadow ring-2 ring-white/60"
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

/** Las flechas de ataque y objetivo entre cartas (y hacia los jugadores). */
function Flechas({ disp }: { disp: Disposicion }) {
	const flechas = useMesa((s) => s.general?.flechas ?? NINGUNA);
	const cartas = useMesa((s) => s.cartas);
	const jugadores = useMesa((s) => s.jugadores);
	if (flechas.length === 0) return null;
	const punto = (e: (typeof flechas)[number]["de"]): { x: number; y: number } | null => {
		if (e.tipo === "carta") {
			const p = cartas[e.id]?.pos;
			return p ? posAPx(disp.campo, p) : null;
		}
		// Un jugador: arriba en el centro (el rival imaginado) o mi vida.
		return { x: disp.campo.w / 2, y: 30 };
	};
	return (
		<svg className="pointer-events-none absolute inset-0 size-full" style={{ zIndex: 6000 }}>
			<defs>
				<marker
					id="punta"
					viewBox="0 0 10 10"
					refX="8"
					refY="5"
					markerWidth="6"
					markerHeight="6"
					orient="auto-start-reverse"
				>
					<path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
				</marker>
			</defs>
			{flechas.map((f, i) => {
				const a = punto(f.de);
				const b = punto(f.a);
				if (!a || !b) return null;
				const color = jugadores[f.deJugador]?.color ?? "#e11d48";
				return (
					<line
						key={i}
						x1={a.x}
						y1={a.y}
						x2={b.x}
						y2={b.y}
						stroke={color}
						strokeWidth={f.tipo === "ataque" ? 4 : 2.5}
						strokeDasharray={f.tipo === "objetivo" ? "8 5" : undefined}
						markerEnd="url(#punta)"
						opacity={0.9}
					/>
				);
			})}
		</svg>
	);
}

/** La vida del rival imaginado del solitario, arriba en el centro. */
function Fantasma({ disp }: { disp: Disposicion }) {
	const { t } = useLang();
	const f = useMesa((s) => s.general?.fantasma ?? null);
	const ver = useUi((s) => s.mesa.fantasma);
	const modo = usePantalla((s) => s.modo);
	if (!f?.visible || !ver) return null;
	const comandantes = Object.entries(f.danoComandante);
	return (
		<div
			className={cn(
				"absolute flex items-center gap-1.5 rounded-full border border-white/15 bg-black/55 px-2 py-1 text-white backdrop-blur-sm",
				modo?.tipo === "flecha" && "cursor-crosshair ring-2 ring-primary",
			)}
			style={{ right: 12, top: 56 }}
			onClick={() => {
				const md = usePantalla.getState().modo;
				if (md?.tipo === "flecha") {
					usePantalla.getState().setModo(null);
					void hacer({
						que: "flecha",
						de: { tipo: "carta", id: md.de },
						a: { tipo: "jugador", id: 0 },
						tipo: md.flecha,
					});
				}
			}}
			onKeyDown={() => {}}
			data-tutorial="mesa-fantasma"
		>
			<Eye className="size-3.5 opacity-70" />
			<span className="text-[10px] opacity-70">{t("Fantasma")}</span>
			<button
				type="button"
				className="px-1 text-sm"
				onClick={() => void hacer({ que: "vidaFantasma", delta: -1 })}
			>
				−
			</button>
			<span className="min-w-6 text-center text-sm font-bold tabular-nums">{f.vida}</span>
			<button
				type="button"
				className="px-1 text-sm"
				onClick={() => void hacer({ que: "vidaFantasma", delta: 1 })}
			>
				+
			</button>
			{comandantes.length > 0 && (
				<span className="text-[10px] opacity-80">{comandantes.map(([, n]) => n).join(" · ")}</span>
			)}
			<span className="sr-only">{disp.tablero.w}</span>
		</div>
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
	// Ya soltada, cuando todas han llegado se quita (o si el movimiento no llega).
	useEffect(() => {
		if (!a?.soltado) return;
		const llegaron = a.ids.every((id) => {
			const c = cartas[id];
			const s = useVivo.getState().soltadas[id];
			return !c || !s || !soltadaSigue(s, c);
		});
		const t = window.setTimeout(
			() => usePantalla.getState().setArrastre(null),
			llegaron ? 0 : MS_SOLTADA,
		);
		return () => window.clearTimeout(t);
	}, [a, cartas]);
	if (!a) return null;
	const dx = a.x - a.x0;
	const dy = a.y - a.y0;
	const { w, h } = a.desde === "campo" || a.alCampo ? disp.carta : disp.cartaPequena;
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
						<ImagenCarta imagen={caraVisible(c)} ancho={244} />
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
						<ImagenCarta imagen={caraVisible(c)} ancho={244} />
					</div>
				);
			})}
		</div>
	);
}
