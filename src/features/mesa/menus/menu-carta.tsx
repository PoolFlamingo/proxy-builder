import {
	ArrowDownToLine,
	ArrowUpToLine,
	Ban,
	Copy,
	Crosshair,
	Crown,
	Eye,
	EyeOff,
	FlipHorizontal2,
	Ghost,
	Hand,
	Hash,
	Highlighter,
	Layers,
	Link2,
	Link2Off,
	Minus,
	MoveRight,
	Paperclip,
	Plus,
	RotateCw,
	Shuffle,
	Skull,
	Sparkles,
	SquareStack,
	Swords,
	Tag,
	Trash2,
	Undo2,
} from "lucide-react";
import type { ReactNode } from "react";
import type { ComoResolver } from "@/bindings/ComoResolver";
import type { IdCarta } from "@/bindings/IdCarta";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { ContenidoContextual } from "@/components/contenido-contextual";
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
import * as op from "../operaciones";
import { pedirNumero, usePantalla } from "../pantalla";

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
	if (soloMira) return <>{children}</>;
	return (
		<ContextMenu onOpenChange={abrir}>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuItem onSelect={() => op.inspeccionar(c.id)} disabled={!c.origen}>
					<Eye />
					{t("Inspeccionar")}
					<Tecla nombre="Inspeccionar" />
				</ContextMenuItem>
				<ContextMenuSeparator />
				{zona === "campo" && <DelCampo c={c} />}
				{zona === "mano" && <DeLaMano c={c} />}
				{zona === "pila" && <DeLaPila c={c} />}
				{zona === "mando" && <DelMando c={c} />}
				{(zona === "cementerio" || zona === "exilio" || zona === "banquillo") && (
					<DeOtraZona c={c} />
				)}
				<ContextMenuSeparator />
				<MoverA c={c} />
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
				<RotateCw />
				{c.girada ? t("Enderezar") : t("Girar")}
				<Tecla nombre="Girar / enderezar" />
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.bocaAbajo(c.id)}>
				{c.bocaAbajo ? <Eye /> : <EyeOff />}
				{c.bocaAbajo ? t("Boca arriba") : t("Boca abajo")}
				<Tecla nombre="Boca abajo" />
			</ContextMenuItem>
			{caras > 1 && (
				<ContextMenuItem onSelect={() => op.transformar(c.id)}>
					<FlipHorizontal2 />
					{t("Transformar")}
					<Tecla nombre="Transformar" />
				</ContextMenuItem>
			)}
			<Contadores c={c} />
			<ContextMenuItem onSelect={() => usePantalla.getState().abrir({ tipo: "pt", carta: c.id })}>
				<Swords />
				{t("Fuerza y resistencia…")}
				<Tecla nombre="Fuerza y resistencia" />
			</ContextMenuItem>
			<ContextMenuItem
				onSelect={() => usePantalla.getState().abrir({ tipo: "etiqueta", carta: c.id })}
			>
				<Tag />
				{t("Etiqueta…")}
			</ContextMenuItem>
			<ContextMenuSeparator />
			{c.adjuntaA !== null ? (
				<ContextMenuItem onSelect={() => op.soltarAdjunto(c.id)}>
					<Link2Off />
					{t("Soltar")}
				</ContextMenuItem>
			) : (
				<ContextMenuItem onSelect={() => op.adjuntarA(c.id)}>
					<Paperclip />
					{t("Adjuntar a…")}
					<Tecla nombre="Adjuntar a…" />
				</ContextMenuItem>
			)}
			<ContextMenuItem onSelect={() => op.flecha(c.id, "ataque")}>
				<Crosshair />
				{t("Atacar / objetivo")}
				<Tecla nombre="Atacar / objetivo" />
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.habilidadEnPila(c.id)}>
				<Sparkles />
				{t("Habilidad a la pila…")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.faseFuera(c.id)}>
				<Ghost />
				{c.faseFuera ? t("Volver de la fase") : t("Fuera de fase")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.noEndereza(c.id)}>
				<Ban />
				{c.noEndereza ? t("Se endereza") : t("No se endereza")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.copiaFicha(c.id)}>
				<Copy />
				{t("Crear una copia ficha")}
			</ContextMenuItem>
			<Resaltar c={c} />
			{(c.cantidad > 1 || varias) && <Monton c={c} varias={varias} />}
		</>
	);
}

function Contadores({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	return (
		<ContextMenuSub>
			<ContextMenuSubTrigger>
				<Hash />
				{t("Contadores")}
			</ContextMenuSubTrigger>
			<ContextMenuSubContent className="w-56 text-xs">
				<ContextMenuItem onSelect={() => op.masUno(c.id, 1)}>
					<Plus />
					{t("Añadir +1/+1")}
					<Tecla nombre="Añadir +1/+1" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.masUno(c.id, -1)}>
					<Minus />
					{t("Quitar +1/+1")}
					<Tecla nombre="Quitar +1/+1" />
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() =>
						usePantalla.getState().abrir({ tipo: "contador", cartas: objetivo(c.id) })
					}
				>
					<Hash />
					{t("Añadir contador…")}
					<Tecla nombre="Añadir contador" />
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => hacer({ que: "multiplicarContadores", carta: c.id, por: 2 })}
					disabled={Object.keys(c.contadores).length === 0}
				>
					<Layers />
					{t("Duplicar los contadores")}
				</ContextMenuItem>
				<ContextMenuSub>
					<ContextMenuSubTrigger>{t("Contadores de palabra clave")}</ContextMenuSubTrigger>
					<ContextMenuSubContent className="text-xs">
						{PALABRAS_CLAVE.map((p) => (
							<ContextMenuItem
								key={p}
								onSelect={() =>
									hacer({
										que: "contadorClave",
										cartas: objetivo(c.id),
										palabra: p,
										si: !c.palabrasClave.includes(p),
									})
								}
							>
								{c.palabrasClave.includes(p) ? "✓ " : ""}
								{p}
							</ContextMenuItem>
						))}
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
								<Trash2 />
								{t("Quitar los de {tipo}", { tipo })}
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
				<Highlighter />
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
					<Trash2 />
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
				<SquareStack />
				{t("Montón")}
			</ContextMenuSubTrigger>
			<ContextMenuSubContent className="w-52 text-xs">
				{varias && (
					<ContextMenuItem onSelect={() => op.amontonar(c.id)}>
						<SquareStack />
						{t("Amontonar las elegidas")}
					</ContextMenuItem>
				)}
				{c.cantidad > 1 && (
					<>
						<ContextMenuItem onSelect={() => op.sacarDelMonton(c.id)}>
							<MoveRight />
							{t("Sacar una")}
							<Tecla nombre="Sacar una del montón" />
						</ContextMenuItem>
						<ContextMenuItem
							onSelect={() => pedirNumero(t("Sacar cuántas"), 1, (n) => op.sacarDelMonton(c.id, n))}
						>
							<MoveRight />
							{t("Sacar varias…")}
						</ContextMenuItem>
						<ContextMenuItem
							onSelect={() =>
								pedirNumero(t("Añadir cuántas (negativo: quitar)"), 1, (n) =>
									hacer({ que: "cambiarMonton", monton: c.id, delta: n }),
								)
							}
						>
							<Plus />
							{t("Añadir o quitar fichas…")}
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => hacer({ que: "separar", monton: c.id })}>
							<Layers />
							{t("Separar")}
						</ContextMenuItem>
					</>
				)}
			</ContextMenuSubContent>
		</ContextMenuSub>
	);
}

function DeLaMano({ c }: { c: VistaCarta }) {
	const { t } = useLang();
	return (
		<>
			<ContextMenuItem onSelect={() => op.aLaPila(c.id)}>
				<Sparkles />
				{t("Lanzar (a la pila)")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.bocaAbajo(c.id)}>
				<EyeOff />
				{t("Jugar boca abajo")}
				<Tecla nombre="Boca abajo" />
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => op.habilidadEnPila(c.id)}>
				<Sparkles />
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
				<Undo2 />
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
				<Crown />
				{t("Lanzar (cuenta para el impuesto)")}
			</ContextMenuItem>
			<ContextMenuItem onSelect={() => mover([c.id], destino("campo", sitio.arriba, yo), "poner")}>
				<Crown />
				{t("Ponerlo en el campo (no cuenta)")}
			</ContextMenuItem>
			<ContextMenuSeparator />
			<div className="px-2 py-1 text-[11px] text-muted-foreground">
				{t("Veces lanzado: {n} · impuesto {coste}", { n: lanzado, coste: impuesto })}
			</div>
			<ContextMenuItem onSelect={() => hacer({ que: "lanzamientos", comandante: c.id, delta: 1 })}>
				<Plus />
				{t("Una vez más")}
			</ContextMenuItem>
			<ContextMenuItem
				disabled={lanzado === 0}
				onSelect={() => hacer({ que: "lanzamientos", comandante: c.id, delta: -1 })}
			>
				<Minus />
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
	return (
		<ContextMenuItem onSelect={() => op.aLaPila(c.id)}>
			<Sparkles />
			{t("Lanzar (a la pila)")}
		</ContextMenuItem>
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
					<Hand />
					{t("A la mano")}
					<Tecla nombre="A la mano" />
				</ContextMenuItem>
			)}
			{zona !== "campo" && (
				<ContextMenuItem onSelect={() => mover(ids(), destino("campo", sitio.arriba, yo))}>
					<ArrowUpToLine />
					{t("Al campo")}
				</ContextMenuItem>
			)}
			{zona !== "cementerio" && (
				<ContextMenuItem onSelect={() => op.aZona(c.id, "cementerio")}>
					<Skull />
					{t("Al cementerio")}
					<Tecla nombre="Al cementerio" />
				</ContextMenuItem>
			)}
			{zona !== "exilio" && (
				<ContextMenuItem onSelect={() => op.aZona(c.id, "exilio")}>
					<Trash2 />
					{t("Al exilio")}
					<Tecla nombre="Al exilio" />
				</ContextMenuItem>
			)}
			<ContextMenuSub>
				<ContextMenuSubTrigger>
					<MoveRight />
					{t("Mover a…")}
				</ContextMenuSubTrigger>
				<ContextMenuSubContent className="w-60 text-xs">
					<ContextMenuItem onSelect={() => op.exiliarBocaAbajo(c.id)}>
						<EyeOff />
						{t("Al exilio boca abajo")}
						<Tecla nombre="Al exilio boca abajo" />
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aZona(c.id, "biblioteca", sitio.arriba)}>
						<ArrowUpToLine />
						{t("Arriba de la biblioteca")}
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aZona(c.id, "biblioteca", sitio.abajo)}>
						<ArrowDownToLine />
						{t("Abajo de la biblioteca")}
						<Tecla nombre="Abajo de la biblioteca" />
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aLaPosicion(c.id)}>
						<Hash />
						{t("En la biblioteca, en la posición…")}
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.barajarDentro(c.id)}>
						<Shuffle />
						{t("Barajar en la biblioteca")}
						<Tecla nombre="Barajar en la biblioteca" />
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aZona(c.id, "mando")}>
						<Crown />
						{t("Al mando")}
					</ContextMenuItem>
					<ContextMenuItem onSelect={() => op.aLaPila(c.id)}>
						<Link2 />
						{t("A la pila")}
					</ContextMenuItem>
				</ContextMenuSubContent>
			</ContextMenuSub>
		</>
	);
}

export type { IdCarta };
