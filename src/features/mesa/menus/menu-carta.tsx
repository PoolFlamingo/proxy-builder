import type { ReactNode } from "react";
import type { ComoResolver } from "@/bindings/ComoResolver";
import type { IdCarta } from "@/bindings/IdCarta";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
import {
	ContextMenu,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuShortcut,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { destino, destinoComun, hacer, mover, objetivo, sitio } from "../acciones";
import { bonita, teclaPara } from "../atajos";
import { COLORES, PALABRAS_CLAVE } from "../colores";
import { conTemporal, nombreContador } from "../contadores";
import { NotasEnMenuCarta } from "../notas-mesa";
import * as op from "../operaciones";
import { pedirNumero, usePantalla } from "../pantalla";
import { campoAjeno } from "../privacidad";

function Tecla({ nombre }: { nombre: string }) {
	const tecla = teclaPara("carta", nombre);
	return tecla ? <ContextMenuShortcut>{tecla}</ContextMenuShortcut> : null;
}

/**
 * **El menú del clic derecho de una carta** (`capturas/pablo/12–13`). Cambia
 * según la zona: campo, mano, pila, mando, cementerio y exilio. Actúa sobre
 * la selección si la carta está en ella.
 */
export function MenuCarta({ c, children }: { c: VistaCarta; children: ReactNode }) {
	const { t } = useLang();
	const zona = c.zona.tipo === "de" ? c.zona.zona : c.zona.zona;
	const abrir = (abierto: boolean) => {
		if (!abierto) return;
		const m = useMesa.getState();
		if (!m.seleccion.includes(c.id)) m.seleccionar([c.id]);
	};
	const soloMira = useRed((s) => s.papel === "espectador");
	// Del campo de otro (protegido) solo se mira o se toma el control.
	const ajena = useMesa((s) => campoAjeno(s, c));
	if (soloMira) return <>{children}</>;
	return (
		<ContextMenu onOpenChange={abrir}>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuItem onSelect={() => op.inspeccionar(c.id)} disabled={!c.origen}>
					<Icono icon="lucide:eye" />
					{t("Inspeccionar")}
					<Tecla nombre="Inspeccionar" />
				</ContextMenuItem>
				<ContextMenuSeparator />
				{ajena && <Control c={c} />}
				{zona === "campo" && !ajena && <DelCampo c={c} />}
				{zona === "mano" && <DeLaMano c={c} />}
				{zona === "pila" && <DeLaPila c={c} />}
				{zona === "mando" && <DelMando c={c} />}
				{(zona === "cementerio" || zona === "exilio" || zona === "banquillo") && (
					<DeOtraZona c={c} />
				)}
				{zona === "campo" && (
					<>
						<ContextMenuSeparator />
						<NotasEnMenuCarta carta={c.id} />
					</>
				)}
				{!ajena && (
					<>
						<ContextMenuSeparator />
						<MoverA c={c} />
					</>
				)}
			</ContenidoContextual>
		</ContextMenu>
	);
}

function DelCampo({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	const varias = useMesa((s) => s.seleccion.length > 1 && s.seleccion.includes(c.id));
	const caras = c.origen?.caras.length ?? 1;
	return (
		<>
			<ContextMenuItem onSelect={() => op.girar(c.id)}>
				<Icono icon="lucide:rotate-cw" />
				{c.girada ? t("Enderezar") : t("Girar")}
				<Tecla nombre="Girar / enderezar" />
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.bocaAbajo(c.id)}>
				{c.bocaAbajo ? <Icono icon="lucide:eye" /> : <Icono icon="lucide:eye-off" />}
				{c.bocaAbajo ? t("Boca arriba") : t("Boca abajo")}
				<Tecla nombre="Boca abajo" />
			</ContextMenuItem>
			{caras > 1 && (
				<ContextMenuItem onSelect={() => op.transformar(c.id)}>
					<Icono icon="lucide:flip-horizontal-2" />
					{t("Transformar")}
					<Tecla nombre="Transformar" />
				</ContextMenuItem>
			)}
			<Control c={c} />
			{c.bocaAbajo && c.controlador === useMesa.getState().yo && <Mostrar c={c} />}
			<Contadores c={c} />
			<ContextMenuItem onSelect={() => usePantalla.getState().abrir({ tipo: "pt", carta: c.id })}>
				<Icono icon="game-icons:crossed-swords" />
				{t("Fuerza y resistencia…")}
				<Tecla nombre="Fuerza y resistencia" />
			</ContextMenuItem>
			<ContextMenuItem
				onSelect={() => usePantalla.getState().abrir({ tipo: "etiqueta", carta: c.id })}
			>
				<Icono icon="lucide:tag" />
				{t("Etiqueta…")}
			</ContextMenuItem>
			<ContextMenuSeparator />
			{c.adjuntaA !== null ? (
				<ContextMenuItem onSelect={() => op.soltarAdjunto(c.id)}>
					<Icono icon="lucide:link-2-off" />
					{t("Soltar")}
				</ContextMenuItem>
			) : (
				<ContextMenuItem onSelect={() => op.adjuntarA(c.id)}>
					<Icono icon="lucide:paperclip" />
					{t("Adjuntar a…")}
					<Tecla nombre="Adjuntar a…" />
				</ContextMenuItem>
			)}
			<ContextMenuItem onSelect={() => op.flecha(c.id, "ataque")}>
				<Icono icon="game-icons:on-target" />
				{t("Atacar / objetivo")}
				<Tecla nombre="Atacar / objetivo" />
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.habilidadEnPila(c.id)}>
				<Icono icon="lucide:sparkles" />
				{t("Habilidad a la pila…")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.faseFuera(c.id)}>
				<Icono icon="game-icons:ghost" />
				{c.faseFuera ? t("Volver de la fase") : t("Fuera de fase")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.noEndereza(c.id)}>
				<Icono icon="lucide:ban" />
				{c.noEndereza ? t("Se endereza") : t("No se endereza")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.copiaFicha(c.id)}>
				<Icono icon="lucide:copy" />
				{t("Crear una copia ficha")}
			</ContextMenuItem>
			<Resaltar c={c} />
			{(c.cantidad > 1 || varias) && <Monton c={c} varias={varias} />}
		</>
	);
}

/**
 * Dar el control de lo mío a otro jugador (regalar) o tomar el de lo que
 * controla otro (robar, para siempre o hasta el final del turno). Va con la
 * carta lo que lleva encima: contadores, equipos, auras y montones.
 */
function Control({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo);
	const jugadores = useMesa((s) => s.jugadores);
	if (yo === null) return null;
	if (c.controlador !== yo) {
		return (
			<>
				<ContextMenuItem onSelect={() => op.tomarControl(objetivo(c.id))}>
					<Icono icon="lucide:arrow-left-right" />
					{t("Tomar el control")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.tomarControl(objetivo(c.id), true)}>
					<Icono icon="lucide:clock" />
					{t("Tomar el control hasta el final del turno")}
				</ContextMenuItem>
			</>
		);
	}
	const otros = Object.values(jugadores).filter((j) => j.id !== yo && j.eliminado === null);
	return (
		<ContextMenuSub>
			<ContextMenuSubTrigger disabled={otros.length === 0}>
				<Icono icon="lucide:gift" />
				{t("Dar el control a…")}
			</ContextMenuSubTrigger>
			<ContextMenuSubContent className="text-xs">
				{otros.map((j) => (
					<ContextMenuItem key={j.id} onSelect={() => op.darControl(objetivo(c.id), j.id)}>
						<span className="size-2.5 rounded-full" style={{ background: j.color }} />
						{j.nombre}
					</ContextMenuItem>
				))}
			</ContextMenuSubContent>
		</ContextMenuSub>
	);
}

function Contadores({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	return (
		<ContextMenuSub>
			<ContextMenuSubTrigger>
				<Icono icon="lucide:hash" />
				{t("Contadores")}
			</ContextMenuSubTrigger>
			<ContextMenuSubContent className="w-56 text-xs">
				<ContextMenuItem onSelect={() => op.masUno(c.id, 1)}>
					<Icono icon="lucide:plus" />
					{t("Añadir +1/+1")}
					<Tecla nombre="Añadir +1/+1" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.masUno(c.id, -1)}>
					<Icono icon="lucide:minus" />
					{t("Quitar +1/+1")}
					<Tecla nombre="Quitar +1/+1" />
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() =>
						usePantalla.getState().abrir({ tipo: "contador", cartas: objetivo(c.id) })
					}
				>
					<Icono icon="lucide:hash" />
					{t("Añadir contador…")}
					<Tecla nombre="Añadir contador" />
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() =>
						usePantalla
							.getState()
							.abrir({ tipo: "contador", cartas: objetivo(c.id), temporal: true })
					}
				>
					<Icono icon="lucide:clock" />
					{t("Añadir contador temporal…")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => usePantalla.getState().abrir({ tipo: "editarContadores", carta: c.id })}
					disabled={Object.keys(c.contadores).length + c.palabrasClave.length === 0}
				>
					<Icono icon="lucide:pencil" />
					{t("Editar contadores…")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => hacer({ que: "multiplicarContadores", carta: c.id, por: 2 })}
					disabled={Object.keys(c.contadores).length === 0}
				>
					<Icono icon="lucide:layers" />
					{t("Duplicar los contadores")}
				</ContextMenuItem>
				<ContextMenuSub>
					<ContextMenuSubTrigger>{t("Contadores de palabra clave")}</ContextMenuSubTrigger>
					<ContextMenuSubContent className="max-h-80 overflow-y-auto text-xs">
						{PALABRAS_CLAVE.map((p) => (
							<ContextMenuItem
								key={p.tipo}
								onSelect={() =>
									hacer({
										que: "contadorClave",
										cartas: objetivo(c.id),
										palabra: p.tipo,
										si: !c.palabrasClave.includes(p.tipo),
									})
								}
							>
								{c.palabrasClave.includes(p.tipo) ? "✓ " : ""}
								{t(p.nombre)}
							</ContextMenuItem>
						))}
						<ContextMenuSeparator />
						<ContextMenuSub>
							<ContextMenuSubTrigger>
								<Icono icon="lucide:clock" />
								{t("Hasta el final del turno")}
							</ContextMenuSubTrigger>
							<ContextMenuSubContent className="max-h-80 overflow-y-auto text-xs">
								{PALABRAS_CLAVE.map((p) => {
									const temporal = conTemporal(p.tipo, true);
									return (
										<ContextMenuItem
											key={temporal}
											onSelect={() =>
												hacer({
													que: "contadorClave",
													cartas: objetivo(c.id),
													palabra: temporal,
													si: !c.palabrasClave.includes(temporal),
												})
											}
										>
											{c.palabrasClave.includes(temporal) ? "✓ " : ""}
											{t(p.nombre)}
										</ContextMenuItem>
									);
								})}
							</ContextMenuSubContent>
						</ContextMenuSub>
					</ContextMenuSubContent>
				</ContextMenuSub>
				{Object.keys(c.contadores).length > 0 && (
					<>
						<ContextMenuSeparator />
						{Object.keys(c.contadores).map((tipo) => (
							<ContextMenuItem
								key={tipo}
								onSelect={() => hacer({ que: "fijarContador", carta: c.id, tipo, valor: 0 })}
							>
								<Icono icon="lucide:trash-2" />
								{t("Quitar los de {tipo}", { tipo: nombreContador(tipo, t) })}
							</ContextMenuItem>
						))}
					</>
				)}
			</ContextMenuSubContent>
		</ContextMenuSub>
	);
}

function Resaltar({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	return (
		<ContextMenuSub>
			<ContextMenuSubTrigger>
				<Icono icon="lucide:highlighter" />
				{t("Resaltar")}
				<ContextMenuShortcut>{bonita("p")}</ContextMenuShortcut>
			</ContextMenuSubTrigger>
			<ContextMenuSubContent className="text-xs">
				<div className="grid grid-cols-8 gap-1 p-1.5">
					{COLORES.map((color, i) => (
						<button
							key={color}
							type="button"
							aria-label={color}
							onClick={() => void op.resaltar(c.id, i)}
							className="size-5 rounded-full ring-1 ring-white/20"
							style={{ background: color }}
						/>
					))}
				</div>
				<ContextMenuItem onSelect={() => op.resaltar(c.id, null)} disabled={c.resaltada === null}>
					<Icono icon="lucide:trash-2" />
					{t("Quitar el resaltado")}
				</ContextMenuItem>
			</ContextMenuSubContent>
		</ContextMenuSub>
	);
}

function Monton({ c, varias }: { c: VistaCarta; varias: boolean }) {
	const { t } = useLang();
	return (
		<ContextMenuSub>
			<ContextMenuSubTrigger>
				<Icono icon="lucide:square-stack" />
				{t("Montón")}
			</ContextMenuSubTrigger>
			<ContextMenuSubContent className="w-52 text-xs">
				{varias && (
					<ContextMenuItem onSelect={() => op.amontonar(c.id)}>
						<Icono icon="lucide:square-stack" />
						{t("Amontonar las elegidas")}
					</ContextMenuItem>
				)}
				{c.cantidad > 1 && (
					<>
						<ContextMenuItem onSelect={() => op.sacarDelMonton(c.id)}>
							<Icono icon="lucide:move-right" />
							{t("Sacar una")}
							<Tecla nombre="Sacar una del montón" />
						</ContextMenuItem>
						<ContextMenuItem
							onSelect={() => pedirNumero(t("Sacar cuántas"), 1, (n) => op.sacarDelMonton(c.id, n))}
						>
							<Icono icon="lucide:move-right" />
							{t("Sacar varias…")}
						</ContextMenuItem>
						<ContextMenuItem
							onSelect={() =>
								pedirNumero(t("Añadir cuántas (negativo: quitar)"), 1, (n) =>
									hacer({ que: "cambiarMonton", monton: c.id, delta: n }),
								)
							}
						>
							<Icono icon="lucide:plus" />
							{t("Añadir o quitar fichas…")}
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => hacer({ que: "separar", monton: c.id })}>
							<Icono icon="lucide:layers" />
							{t("Separar")}
						</ContextMenuItem>
					</>
				)}
			</ContextMenuSubContent>
		</ContextMenuSub>
	);
}

/**
 * Enseñar una carta que solo ves tú (de la mano o boca abajo en el campo) a
 * todos —también a quien solo mira— o a un jugador, y volver a taparla.
 */
function Mostrar({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo);
	// El selector devuelve el objeto del store (estable); el filtro va fuera: un array
	// nuevo en cada lectura haría saltar «Maximum update depth exceeded».
	const jugadores = useMesa((s) => s.jugadores);
	if (yo === null) return null;
	const otros = Object.values(jugadores).filter((j) => j.id !== yo);
	return (
		<ContextMenuSub>
			<ContextMenuSubTrigger>
				<Icono icon="lucide:eye" />
				{t("Mostrar…")}
				<Tecla nombre="Mostrar a todos / dejar de mostrar" />
			</ContextMenuSubTrigger>
			<ContextMenuSubContent className="text-xs">
				<ContextMenuItem onSelect={() => op.mostrar(c.id)}>{t("A todos")}</ContextMenuItem>
				{otros.map((j) => (
					<ContextMenuItem key={j.id} onSelect={() => op.mostrar(c.id, j.id)}>
						<span className="size-2.5 rounded-full" style={{ background: j.color }} />
						{j.nombre}
					</ContextMenuItem>
				))}
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => op.dejarDeMostrar(c.id)}>
					<Icono icon="lucide:eye-off" />
					{t("Dejar de mostrar")}
				</ContextMenuItem>
			</ContextMenuSubContent>
		</ContextMenuSub>
	);
}

function DeLaMano({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	return (
		<>
			<Mostrar c={c} />
			<ContextMenuItem onSelect={() => op.aLaPila(c.id)}>
				<Icono icon="lucide:sparkles" />
				{t("Lanzar (a la pila)")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.bocaAbajo(c.id)}>
				<Icono icon="lucide:eye-off" />
				{t("Jugar boca abajo")}
				<Tecla nombre="Boca abajo" />
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.habilidadEnPila(c.id)}>
				<Icono icon="lucide:sparkles" />
				{t("Habilidad a la pila…")}
			</ContextMenuItem>
		</>
	);
}

const RESOLVER: { como: ComoResolver; nombre: string }[] = [
	{ como: "permanente", nombre: paraTraducir("Al campo") },
	{ como: "hechizo", nombre: paraTraducir("Al cementerio") },
	{ como: "mitadAventura", nombre: paraTraducir("Al exilio (aventura)") },
	{ como: "mitadOmen", nombre: paraTraducir("Barajarla (omen)") },
];

function DeLaPila({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	return (
		<>
			<ContextMenuItem onSelect={() => hacer({ que: "resolver", carta: c.id, como: "auto" })}>
				<Icono icon="lucide:undo-2" />
				{t("Resolver")}
			</ContextMenuItem>
			{!c.habilidad && (
				<ContextMenuSub>
					<ContextMenuSubTrigger>{t("Resolver de otra forma")}</ContextMenuSubTrigger>
					<ContextMenuSubContent className="text-xs">
						{RESOLVER.map((r) => (
							<ContextMenuItem
								key={r.como}
								onSelect={() => hacer({ que: "resolver", carta: c.id, como: r.como })}
							>
								{t(r.nombre)}
							</ContextMenuItem>
						))}
					</ContextMenuSubContent>
				</ContextMenuSub>
			)}
		</>
	);
}

function DelMando({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const lanzado = useMesa((s) => s.jugadores[c.dueno]?.lanzamientos[c.id] ?? 0);
	const sinCoste = useMesa((s) => s.jugadores[c.dueno]?.sinCosteExtra.includes(c.id) ?? false);
	const impuesto = sinCoste ? 0 : lanzado * 2;
	return (
		<>
			<ContextMenuItem onSelect={() => mover([c.id], destinoComun("pila"), "lanzar")}>
				<Icono icon="game-icons:crown" />
				{t("Lanzar (cuenta para el impuesto)")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => mover([c.id], destino("campo", sitio.arriba, yo), "poner")}>
				<Icono icon="game-icons:crown" />
				{t("Ponerlo en el campo (no cuenta)")}
			</ContextMenuItem>
			<ContextMenuSeparator />
			<div className="px-2 py-1 text-[11px] text-muted-foreground">
				{t("Veces lanzado: {n} · impuesto {coste}", { n: lanzado, coste: impuesto })}
			</div>
			<ContextMenuItem onSelect={() => hacer({ que: "lanzamientos", comandante: c.id, delta: 1 })}>
				<Icono icon="lucide:plus" />
				{t("Una vez más")}
			</ContextMenuItem>
			<ContextMenuItem
				disabled={lanzado === 0}
				onSelect={() => hacer({ que: "lanzamientos", comandante: c.id, delta: -1 })}
			>
				<Icono icon="lucide:minus" />
				{t("Una vez menos")}
			</ContextMenuItem>
			<ContextMenuItem
				onSelect={() => hacer({ que: "sinCosteExtra", comandante: c.id, si: !sinCoste })}
			>
				{sinCoste ? "✓ " : ""}
				{t("Sin coste extra")}
			</ContextMenuItem>
		</>
	);
}

function DeOtraZona({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	const zona = c.zona.tipo === "de" ? c.zona.zona : null;
	const jugador = c.zona.tipo === "de" ? c.zona.jugador : null;
	const nombre =
		zona === "cementerio"
			? t("Cementerio")
			: zona === "exilio"
				? t("Exilio")
				: zona === "banquillo"
					? t("Banquillo")
					: "";
	return (
		<>
			<ContextMenuItem onSelect={() => op.aLaPila(c.id)}>
				<Icono icon="lucide:sparkles" />
				{t("Lanzar (a la pila)")}
			</ContextMenuItem>
			{zona !== null && jugador !== null && (
				<ContextMenuItem
					onSelect={() => usePantalla.getState().abrir({ tipo: "zona", jugador, zona })}
				>
					<Icono icon="lucide:grid-3x3" />
					{t("Ver todas las cartas ({zona})", { zona: nombre })}
				</ContextMenuItem>
			)}
		</>
	);
}

/** «Mover a…»: todas las zonas, con las teclas de Playgroup. */
function MoverA({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const ids = () => objetivo(c.id);
	const zona = c.zona.tipo === "de" ? c.zona.zona : null;
	return (
		<>
			{zona !== "mano" && (
				<ContextMenuItem onSelect={() => op.aZona(c.id, "mano")}>
					<Icono icon="lucide:hand" />
					{t("A la mano")}
					<Tecla nombre="A la mano" />
				</ContextMenuItem>
			)}
			{zona !== "campo" && (
				<ContextMenuItem onSelect={() => mover(ids(), destino("campo", sitio.arriba, yo))}>
					<Icono icon="lucide:arrow-up-to-line" />
					{t("Al campo")}
				</ContextMenuItem>
			)}
			{zona !== "cementerio" && (
				<ContextMenuItem onSelect={() => op.aZona(c.id, "cementerio")}>
					<Icono icon="game-icons:tombstone" />
					{t("Al cementerio")}
					<Tecla nombre="Al cementerio" />
				</ContextMenuItem>
			)}
			{zona !== "exilio" && (
				<ContextMenuItem onSelect={() => op.aZona(c.id, "exilio")}>
					<Icono icon="lucide:trash-2" />
					{t("Al exilio")}
					<Tecla nombre="Al exilio" />
				</ContextMenuItem>
			)}
			<ContextMenuSub>
				<ContextMenuSubTrigger>
					<Icono icon="lucide:move-right" />
					{t("Mover a…")}
				</ContextMenuSubTrigger>
				<ContextMenuSubContent className="w-60 text-xs">
					<ContextMenuItem onSelect={() => op.exiliarBocaAbajo(c.id)}>
						<Icono icon="lucide:eye-off" />
						{t("Al exilio boca abajo")}
						<Tecla nombre="Al exilio boca abajo" />
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aZona(c.id, "biblioteca", sitio.arriba)}>
						<Icono icon="lucide:arrow-up-to-line" />
						{t("Arriba de la biblioteca")}
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aZona(c.id, "biblioteca", sitio.abajo)}>
						<Icono icon="lucide:arrow-down-to-line" />
						{t("Abajo de la biblioteca")}
						<Tecla nombre="Abajo de la biblioteca" />
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aLaPosicion(c.id)}>
						<Icono icon="lucide:hash" />
						{t("En la biblioteca, en la posición…")}
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.barajarDentro(c.id)}>
						<Icono icon="lucide:shuffle" />
						{t("Barajar en la biblioteca")}
						<Tecla nombre="Barajar en la biblioteca" />
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aZona(c.id, "mando")}>
						<Icono icon="game-icons:crown" />
						{t("Al mando")}
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aLaPila(c.id)}>
						<Icono icon="lucide:link-2" />
						{t("A la pila")}
					</ContextMenuItem>
				</ContextMenuSubContent>
			</ContextMenuSub>
		</>
	);
}

export type { IdCarta };
