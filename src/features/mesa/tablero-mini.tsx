import { Hand, Heart, Languages, Library, Reply, Skull, Swords } from "lucide-react";
import { memo } from "react";
import type { Equipo } from "@/bindings/Equipo";
import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { VistaJugador } from "@/bindings/VistaJugador";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { useLang } from "@/components/language-provider";
import { ContextMenu, ContextMenuItem, ContextMenuTrigger } from "@/components/ui/context-menu";
import { urlMesa } from "@/lib/image-url";
import { cn } from "@/lib/utils";
import { cartasDe, cuantasEn, NINGUNA, useMesa } from "@/stores/mesa-store";
import { useOriginales } from "@/stores/originales-store";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { enVuelo, origenDe, useVivo } from "@/stores/vivo-store";
import { zonaComun, zonaDe } from "./acciones";
import { cartaDeArribaRevelada } from "./arriba-biblioteca";
import { Bocadillo, InsigniaSenal } from "./chat";
import { atacantesDe } from "./cuentas-flechas";
import { abanico, disponer, posAPx } from "./disposicion";
import { senalar } from "./en-vivo/emitir";
import { FantasmasMini } from "./en-vivo/fantasmas";
import { caraVisible, DorsoGenerico, ImagenCarta } from "./imagen-carta";
import { Insignias } from "./insignias";
import { igualesLista, useMesaLenta } from "./lento";
import { mirarJugador } from "./mirar";
import { elegirCarta, elegirJugador } from "./objetivo";
import { alternarOriginales } from "./originales";
import { usePantalla } from "./pantalla";
import { puestosDelCampo } from "./puestos-campo";
import { Latencia } from "./red/estado-red";

/** Alto de la cabecera (nombre y vida) de una miniatura, en px. */
const CABECERA = 20;
/** Por debajo de este alto, la miniatura no pinta el campo, solo los números. */
const ALTO_CON_CAMPO = 64;
/**
 * A partir de este ancho y alto la otra mesa se pinta **a la misma escala que
 * la propia** (`LecturaGrande`): en «dos tableros» cada uno ocupa media
 * pantalla y no tiene sentido enseñar cartas de 40 px.
 */
const ANCHO_GRANDE = 420;
const ALTO_GRANDE = 340;

type DatosMini = {
	j: VistaJugador | undefined;
	campo: VistaCarta[];
	mano: number;
	/** Las cartas de la mano que este mirador ve (la de un compañero); el resto, reversos. */
	manoVista: VistaCarta[];
	biblioteca: number;
	/** La de arriba, si está boca arriba (revelada). */
	arribaBiblioteca: VistaCarta | null;
	/** La de arriba de cada zona a la vista (la última de la lista) y cuántas hay. */
	cementerio: ArribaMini;
	exilio: ArribaMini;
	mando: ArribaMini;
	activo: boolean;
	equipo: Equipo | undefined;
};

type ArribaMini = { carta: VistaCarta | null; n: number };

/** Lo que enseña la miniatura de `jugador`, sacado de la mesa. */
export function datosMini(
	m: Pick<ReturnType<typeof useMesa.getState>, "jugadores" | "zonas" | "cartas" | "general">,
	jugador: IdJugador,
): DatosMini {
	const j = m.jugadores[jugador];
	const campo: VistaCarta[] = [];
	for (const id of cartasDe(m, zonaDe(jugador, "campo"))) {
		const c = m.cartas[id];
		if (c) campo.push(c);
	}
	const manoVista: VistaCarta[] = [];
	for (const id of cartasDe(m, zonaDe(jugador, "mano"))) {
		const c = m.cartas[id];
		if (c) manoVista.push(c);
	}
	const arriba = (zona: "cementerio" | "exilio" | "mando"): ArribaMini => {
		const ids = cartasDe(m, zonaDe(jugador, zona));
		const id = ids[ids.length - 1];
		return {
			carta: id !== undefined ? (m.cartas[id] ?? null) : null,
			n: cuantasEn(m, zonaDe(jugador, zona)),
		};
	};
	return {
		j,
		campo,
		cementerio: arriba("cementerio"),
		exilio: arriba("exilio"),
		mando: arriba("mando"),
		mano: cuantasEn(m, zonaDe(jugador, "mano")),
		manoVista,
		biblioteca: cuantasEn(m, zonaDe(jugador, "biblioteca")),
		arribaBiblioteca: cartaDeArribaRevelada(m, jugador),
		activo: m.general?.turno.activo === jugador,
		equipo: j?.equipo != null ? m.general?.equipos.find((e) => e.id === j.equipo) : undefined,
	};
}

function igualArriba(a: ArribaMini, b: ArribaMini): boolean {
	return a.carta === b.carta && a.n === b.n;
}

/** Dos lecturas de la misma miniatura que se pintarían igual. */
export function igualMini(a: DatosMini, b: DatosMini): boolean {
	return (
		a.j === b.j &&
		a.mano === b.mano &&
		igualesLista(a.manoVista, b.manoVista) &&
		a.biblioteca === b.biblioteca &&
		a.arribaBiblioteca === b.arribaBiblioteca &&
		igualArriba(a.cementerio, b.cementerio) &&
		igualArriba(a.exilio, b.exilio) &&
		igualArriba(a.mando, b.mando) &&
		a.activo === b.activo &&
		a.equipo === b.equipo &&
		igualesLista(a.campo, b.campo)
	);
}

/**
 * **El tablero de otro jugador, en pequeño**: su nombre y vida, cuántas cartas
 * tiene en la mano y la biblioteca y, debajo, su campo a escala. No se puede
 * tocar nada: pulsarlo pasa a mirar a ese jugador. Se pone al día como mucho
 * 10 veces por segundo y sin animaciones, para que 9 miniaturas no frenen el
 * tablero grande.
 */
export const TableroMini = memo(function TableroMini({
	jugador,
	w,
	h,
}: {
	jugador: IdJugador;
	w: number;
	h: number;
}) {
	const { t } = useLang();
	const d = useMesaLenta((m) => datosMini(m, jugador), igualMini);
	const enRed = useRed((s) => s.papel !== null);
	const senalada = useVivo((s) => s.senales.some((x) => x.a === jugador));
	const originales = useOriginales((s) => s.modos[jugador]);
	const modo = usePantalla((s) => s.modo);
	// Cuántas flechas de ataque le apuntan (a él o a sus cartas).
	const atacantes = useMesa((s) => atacantesDe(s.general?.flechas ?? NINGUNA, s.cartas, jugador));
	// Con algo en la pila y el turno de quien miro, esta mesa puede responder.
	const respondible = useMesa(
		(s) =>
			!enRed &&
			!d.activo &&
			s.yo !== null &&
			s.yo === s.general?.turno.activo &&
			cuantasEn(s, zonaComun("pila")) > 0,
	);
	const j = d.j;
	if (!j) return null;
	const color = d.equipo?.color ?? j.color;
	const vida = d.equipo?.vidaCompartida ?? j.vida;
	const eliminado = j.eliminado !== null;
	const conCampo = h >= ALTO_CON_CAMPO;
	const grande = w >= ANCHO_GRANDE && h >= ALTO_GRANDE;
	const campo = { x: 0, y: CABECERA, w, h: Math.max(0, h - CABECERA) };
	const ancho = Math.max(14, Math.min(44, w / 10));
	const alto = (ancho * 88) / 63;
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>
				<button
					type="button"
					onClick={(e) => {
						// Eligiendo objetivo: la flecha va a este jugador (adjuntar pide una carta,
						// y pulsar el fondo no cambia de mesa a mitad de elegir).
						if (modo) {
							elegirJugador(jugador, t);
							return;
						}
						// Alt + clic: un aro de aviso sobre el jugador, en todas las pantallas.
						if (e.altKey && enRed) senalar(null, jugador);
						else void mirarJugador(jugador);
					}}
					aria-label={j.nombre}
					className={cn(
						"relative isolate block size-full overflow-hidden rounded-md border-2 bg-black/50 text-left text-white",
						d.activo && "ring-2 ring-primary",
						eliminado && "opacity-50",
						senalada && "animate-pulse ring-2 ring-primary",
						modo?.tipo === "flecha" && "cursor-crosshair ring-2 ring-primary",
					)}
					style={{ borderColor: color }}
				>
					{conCampo && j.tapete && urlMesa(j.tapete, 745) && (
						<img
							src={urlMesa(j.tapete, 745) ?? undefined}
							alt=""
							draggable={false}
							className="pointer-events-none absolute inset-0 -z-10 size-full object-cover opacity-70"
						/>
					)}
					<Bocadillo jugador={jugador} className="absolute bottom-1 left-1" />
					{respondible && !modo && (
						<span className="pointer-events-none absolute inset-x-0 bottom-1 z-[9500] flex justify-center">
							<span className="flex animate-pulse items-center gap-1 rounded-full bg-primary px-2 py-0.5 text-[10px] font-semibold text-primary-foreground shadow-lg">
								<Reply className="size-3" />
								{t("Hay algo en la pila: ¿responder?")}
							</span>
						</span>
					)}
					<span
						className="absolute inset-x-0 top-0 flex items-center gap-1.5 px-1.5 text-[11px] leading-none"
						style={{ height: CABECERA, background: `${color}66` }}
					>
						{eliminado && <Skull className="size-3 shrink-0" />}
						<span className="min-w-0 flex-1 truncate font-medium">{j.nombre}</span>
						<InsigniaSenal jugador={jugador} />
						{enRed && <Latencia id={jugador} />}
						{originales && <Languages className="size-3 shrink-0 text-white/60" />}
						{atacantes > 0 && (
							<span
								className="flex shrink-0 items-center gap-0.5 rounded-sm bg-rose-600 px-1 py-0.5 font-semibold tabular-nums"
								aria-label={t("Ataques contra {nombre}: {cuantos}", {
									nombre: j.nombre,
									cuantos: atacantes,
								})}
							>
								<Swords className="size-3" />
								{atacantes}
							</span>
						)}
						<span className="flex shrink-0 items-center gap-0.5 tabular-nums">
							<Heart className="size-3" />
							{vida}
						</span>
						{w >= 120 && (
							<>
								<span
									className="flex shrink-0 items-center gap-0.5 tabular-nums"
									aria-label={t("Mano")}
								>
									<Hand className="size-3" />
									{d.mano}
								</span>
								<span
									className="flex shrink-0 items-center gap-0.5 tabular-nums"
									aria-label={t("Biblioteca")}
								>
									<Library className="size-3" />
									{d.biblioteca}
								</span>
							</>
						)}
					</span>
					{conCampo && grande && <LecturaGrande d={d} w={w} h={h} />}
					{conCampo && !grande && d.mano > 0 && <ManoMini d={d} w={w} ancho={ancho} />}
					{conCampo && !grande && <ZonasMini d={d} w={w} h={h} ancho={ancho} />}
					{conCampo &&
						!grande &&
						d.campo.map((c) => {
							if (!c.pos || c.adjuntaA !== null || c.agrupadaBajo !== null) return null;
							// Si alguien la está arrastrando, se ve el arrastre y no su sitio de antes.
							if (enVuelo(useVivo.getState(), c.id, () => origenDe(c))) return null;
							const p = posAPx(campo, c.pos);
							return (
								<span
									key={c.id}
									className="absolute block"
									style={{
										left: p.x - ancho / 2,
										top: p.y - alto / 2,
										width: ancho,
										height: alto,
										zIndex: c.z,
										transform: c.girada ? "rotate(90deg)" : undefined,
									}}
									onPointerEnter={() => useMesa.getState().setSobre(c.id)}
									onPointerLeave={() => {
										if (useMesa.getState().sobre === c.id) useMesa.getState().setSobre(null);
									}}
								>
									<ImagenCarta imagen={caraVisible(c)} ancho={146} />
								</span>
							);
						})}
					{conCampo && <FantasmasMini jugador={jugador} campo={campo} ancho={ancho} alto={alto} />}
				</button>
			</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuItem onSelect={() => alternarOriginales(jugador)}>
					<Languages />
					{originales ? "✓ " : ""}
					{t("Cartas originales en mi idioma")}
				</ContextMenuItem>
			</ContenidoContextual>
		</ContextMenu>
	);
});

/** Tope de reversos que se dibujan en la franja de la mano de una miniatura. */
const MAX_MANO_MINI = 20;

/**
 * La mano de un jugador en pequeño, abajo de su miniatura: **siempre se ve**,
 * con el reverso del deck de ese jugador (el elegido o el genérico) aunque la
 * carta sea de doble cara, para que nadie sepa qué tiene. Solo las cartas que
 * este mirador puede ver de verdad (la mano de un compañero de equipo) salen
 * boca arriba; esas las decide el motor.
 */
function ManoMini({ d, w, ancho }: { d: DatosMini; w: number; ancho: number }) {
	const n = Math.min(d.mano, MAX_MANO_MINI);
	const cw = Math.max(12, Math.min(ancho, 30));
	const ch = (cw * 88) / 63;
	const franja = ch * 0.62;
	// El mismo abanico que la mano grande, a escala: el arco y el giro se
	// reducen con el tamaño de la carta para que se note sin torcerla mucho.
	const escala = ch / 90;
	const pos = abanico({ x: 0, y: 0, w: w - 8, h: ch }, n, { w: cw, h: ch }).map((p) => ({
		x: p.x + 4,
		y: ch / 2 + (p.y - ch / 2) * escala,
		giro: p.giro,
	}));
	const dorso = d.j?.dorso ?? null;
	const abiertas = d.manoVista.length > 0;
	return (
		<span
			className="pointer-events-none absolute inset-x-0 bottom-0 z-[9000] block bg-black/55"
			style={{ height: franja + 4 }}
			data-mano={abiertas ? "abierta" : "oculta"}
		>
			{Array.from({ length: n }, (_, i) => {
				// De la mano abierta, las que se ven; si faltan, reversos.
				const c = d.manoVista[i];
				return (
					<span
						key={c?.id ?? `r${i}`}
						className="absolute block overflow-hidden rounded-[2px]"
						style={{
							left: (pos[i]?.x ?? 0) - cw / 2,
							top: (pos[i]?.y ?? ch / 2) - ch / 2 + 2,
							width: cw,
							height: ch,
							transform: `rotate(${pos[i]?.giro ?? 0}deg)`,
						}}
					>
						{c ? (
							<ImagenCarta imagen={caraVisible(c)} ancho={146} />
						) : dorso ? (
							<ImagenCarta imagen={dorso} ancho={146} />
						) : (
							<DorsoGenerico />
						)}
					</span>
				);
			})}
		</span>
	);
}

/**
 * Las zonas de un jugador en pequeño, como en el tablero grande: el mando, la
 * biblioteca (con su reverso) y el cementerio y el exilio con la carta de
 * arriba boca arriba (si se ve), cada una con cuántas tiene. En una
 * miniatura ancha van en fila abajo a la derecha, con su nombre, como en tu
 * tablero; en una estrecha, en columna a la derecha.
 */
function ZonasMini({ d, w, h, ancho }: { d: DatosMini; w: number; h: number; ancho: number }) {
	const { t } = useLang();
	const zonas: {
		clave: string;
		nombre: string;
		carta: VistaCarta | null;
		n: number;
		dorso?: boolean;
	}[] = [];
	if (d.mando.n > 0) zonas.push({ clave: "mando", nombre: t("Mando"), ...d.mando });
	zonas.push({
		clave: "biblioteca",
		nombre: t("Biblioteca"),
		carta: d.arribaBiblioteca,
		n: d.biblioteca,
		dorso: d.arribaBiblioteca === null,
	});
	zonas.push({ clave: "cementerio", nombre: t("Cementerio"), ...d.cementerio });
	zonas.push({ clave: "exilio", nombre: t("Exilio"), ...d.exilio });
	const enFila = w >= FILA_ZONAS;
	const altoMax = (Math.max(14, Math.min(ancho * 1.1, 40)) * 88) / 63;
	let th: number;
	if (enFila) {
		th = Math.min(altoMax, (h - CABECERA) * 0.3);
	} else {
		// Lo que queda entre la cabecera y la franja de la mano.
		const cwMano = Math.max(12, Math.min(ancho, 30));
		const franja = d.mano > 0 ? ((cwMano * 88) / 63) * 0.62 + 4 : 0;
		const hueco = h - CABECERA - franja - 6;
		th = Math.min(altoMax, (hueco - (zonas.length - 1) * 3) / zonas.length);
	}
	if (th < 14) return null;
	const tw = (th * 63) / 88;
	const conNombre = enFila && th >= 34;
	return (
		<span
			className={cn(
				"pointer-events-none absolute z-[9100] flex gap-[3px]",
				enFila ? "right-1.5 bottom-1.5 items-end" : "right-1 flex-col",
			)}
			style={enFila ? undefined : { top: CABECERA + 3 }}
		>
			{zonas.map((z) => (
				<span key={z.clave} className="flex flex-col items-center gap-0.5">
					{conNombre && (
						<span className="text-[8px] leading-none whitespace-nowrap text-white/70">
							{z.nombre}
						</span>
					)}
					<span
						className="pointer-events-auto relative block overflow-hidden rounded-[2px] border border-dashed border-white/25 bg-black/50"
						style={{ width: tw, height: th }}
						aria-label={`${z.nombre}: ${z.n}`}
						data-zona={z.clave}
						onPointerEnter={() => z.carta && useMesa.getState().setSobre(z.carta.id)}
						onPointerLeave={() => {
							if (z.carta && useMesa.getState().sobre === z.carta.id)
								useMesa.getState().setSobre(null);
						}}
					>
						{z.dorso && z.n > 0 ? (
							d.j?.dorso ? (
								<ImagenCarta imagen={d.j.dorso} ancho={146} />
							) : (
								<DorsoGenerico />
							)
						) : z.carta ? (
							<ImagenCarta imagen={caraVisible(z.carta)} ancho={146} />
						) : null}
						{z.n > 0 && (
							<span className="absolute right-0 bottom-0 rounded-tl-[2px] bg-black/75 px-0.5 text-[9px] leading-none tabular-nums">
								{z.n}
							</span>
						)}
					</span>
				</span>
			))}
		</span>
	);
}

/** Desde este ancho, las zonas de una miniatura van en fila (como en tu tablero). */
const FILA_ZONAS = 400;

/**
 * **La otra mesa, a la escala de la propia** (solo se mira): el campo con las
 * cartas del mismo tamaño que en el tablero grande (`disponer`, `puestosDelCampo`:
 * la misma geometría), con sus contadores y su fuerza y resistencia, la mano
 * en abanico (siempre sus reversos, salvo lo que este mirador puede ver), y
 * las zonas con la carta de arriba. Pulsarla pasa a mirar a ese jugador.
 */
function LecturaGrande({ d, w, h }: { d: DatosMini; w: number; h: number }) {
	const { t } = useLang();
	const tamano = useUi((s) => s.mesa.tamanoCartas);
	const alto = Math.max(0, h - CABECERA);
	const disp = disponer(w, alto, tamano);
	const dorso = d.j?.dorso ?? null;
	const reverso = dorso ? <ImagenCarta imagen={dorso} ancho={244} /> : <DorsoGenerico />;
	const cartas: Record<number, VistaCarta> = {};
	for (const c of d.campo) cartas[c.id] = c;
	const { puestos } = puestosDelCampo(
		d.campo.map((c) => c.id),
		cartas,
		disp,
	);
	// La mano: reversos, y boca arriba las que este mirador ve de verdad.
	const n = Math.min(d.mano, 30);
	const pos = abanico(disp.mano, n, disp.cartaPequena);
	const zonas: {
		clave: string;
		nombre: string;
		carta: VistaCarta | null;
		n: number;
		dorso?: boolean;
	}[] = [];
	if (d.mando.n > 0) zonas.push({ clave: "mando", nombre: t("Mando"), ...d.mando });
	zonas.push({
		clave: "biblioteca",
		nombre: t("Biblioteca"),
		carta: d.arribaBiblioteca,
		n: d.biblioteca,
		dorso: d.arribaBiblioteca === null,
	});
	zonas.push({ clave: "cementerio", nombre: t("Cementerio"), ...d.cementerio });
	zonas.push({ clave: "exilio", nombre: t("Exilio"), ...d.exilio });
	return (
		<div className="pointer-events-none absolute inset-x-0 bottom-0" style={{ top: CABECERA }}>
			{d.campo.map((c) => {
				const p = puestos.get(c.id);
				if (!p) return null;
				if (enVuelo(useVivo.getState(), c.id, () => origenDe(c))) return null;
				const { w: cw, h: ch } = disp.carta;
				return (
					<span
						key={c.id}
						className="pointer-events-auto absolute block"
						style={{
							left: p.x - cw / 2,
							top: p.y - ch / 2,
							width: cw,
							height: ch,
							zIndex: p.z ?? c.z,
						}}
						onClick={(e) => {
							// Con una flecha o un adjuntar en marcha, la carta es el objetivo.
							if (elegirCarta(c, t)) e.stopPropagation();
						}}
						onPointerEnter={() => useMesa.getState().setSobre(c.id)}
						onPointerLeave={() => {
							if (useMesa.getState().sobre === c.id) useMesa.getState().setSobre(null);
						}}
					>
						<span
							className="absolute inset-0 block overflow-hidden rounded-[5%] shadow-[0_3px_10px_rgba(0,0,0,0.55)]"
							style={{ transform: c.girada ? "rotate(90deg)" : undefined }}
						>
							<ImagenCarta imagen={caraVisible(c)} ancho={244} nombre={c.origen?.nombre} />
						</span>
						<Insignias c={c} yo={null} />
					</span>
				);
			})}
			{Array.from({ length: n }, (_, i) => {
				const p = pos[i];
				if (!p) return null;
				const visible = d.manoVista[i];
				return (
					<span
						key={visible?.id ?? `r${i}`}
						className="absolute block overflow-hidden rounded-[5%] shadow-[0_3px_10px_rgba(0,0,0,0.55)]"
						style={{
							left: p.x - disp.cartaPequena.w / 2,
							top: p.y - disp.cartaPequena.h / 2,
							width: disp.cartaPequena.w,
							height: disp.cartaPequena.h,
							transform: `rotate(${p.giro}deg)`,
							zIndex: 1000 + i,
						}}
					>
						{visible ? <ImagenCarta imagen={caraVisible(visible)} ancho={244} /> : reverso}
					</span>
				);
			})}
			{zonas.map((z) => {
				const r = disp.zonas[z.clave as keyof typeof disp.zonas];
				return r ? (
					<span
						key={z.clave}
						className="pointer-events-auto absolute flex flex-col items-center"
						style={{ left: r.x, top: r.y - 16, width: r.w, height: r.h + 16 }}
						onPointerEnter={() => z.carta && useMesa.getState().setSobre(z.carta.id)}
						onPointerLeave={() => {
							if (z.carta && useMesa.getState().sobre === z.carta.id)
								useMesa.getState().setSobre(null);
						}}
					>
						<span className="h-4 text-[10px] whitespace-nowrap text-white/80">
							{z.nombre}
							{z.n > 0 ? ` · ${z.n}` : ""}
						</span>
						<span className="relative block w-full flex-1 overflow-hidden rounded-[5%] border border-dashed border-white/25 bg-black/20">
							{z.dorso && z.n > 0 ? (
								reverso
							) : z.carta ? (
								<ImagenCarta imagen={caraVisible(z.carta)} ancho={244} />
							) : null}
						</span>
					</span>
				) : null;
			})}
		</div>
	);
}
