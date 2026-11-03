import type { DeckRevisado } from "@/bindings/DeckRevisado";
import type { EleccionPasar } from "@/bindings/EleccionPasar";
import type { ResolucionPasar } from "@/bindings/ResolucionPasar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { useFormato } from "@/lib/formato";
import { cn } from "@/lib/utils";
import { type Elegido, opcionesDe, pesar } from "./cuentas";

/**
 * **La oferta de decks**, para elegir qué se recibe de cada uno: con proxies,
 * solo las cartas originales o no, y —si ya está aquí— qué hacer con el que ya
 * hay. La usan «Recibir» (lo que manda otro equipo) y la importación de un
 * `.pbdeck`. Solo pinta y avisa de lo elegido (`cuentas.ts` decide).
 */
export function OfertaDecks({
	oferta,
	elegidos,
	onCambio,
}: {
	oferta: DeckRevisado[];
	elegidos: Elegido[];
	onCambio: (i: number, e: Elegido) => void;
}) {
	return (
		<ul className="flex flex-col gap-2.5 p-1">
			{oferta.map((d, i) => (
				<li key={i}>
					<TarjetaDeck
						deck={d}
						elegido={elegidos[i] ?? { eleccion: "conProxies", resolucion: null }}
						onCambio={(e) => onCambio(i, e)}
					/>
				</li>
			))}
		</ul>
	);
}

function TarjetaDeck({
	deck,
	elegido,
	onCambio,
}: {
	deck: DeckRevisado;
	elegido: Elegido;
	onCambio: (e: Elegido) => void;
}) {
	const { t, lang } = useLang();
	const movil = useFormato().tamano === "movil";
	const d = deck.deck;
	const acepta = elegido.eleccion !== "no";
	const nombreOpcion = (o: EleccionPasar) =>
		o === "conProxies"
			? d.proxies
				? t("Con proxies ({peso})", { peso: pesar(d.tamano, lang) })
				: t("Recibirlo ({peso})", { peso: pesar(d.tamano, lang) })
			: o === "soloOriginales"
				? t("Solo cartas originales ({peso})", { peso: pesar(d.tamanoOriginales, lang) })
				: t("No");
	return (
		<div className={cn("rounded-xl border p-2.5", !acepta && "opacity-70")}>
			<div className="flex gap-2.5">
				{d.portada ? (
					<img
						src={`data:image/jpeg;base64,${d.portada}`}
						alt=""
						className="h-14 w-10 shrink-0 rounded-[4px] object-cover"
					/>
				) : (
					<div className="flex h-14 w-10 shrink-0 items-center justify-center rounded-[4px] bg-muted text-muted-foreground">
						<Icono icon="lucide:layers" className="size-4" />
					</div>
				)}
				<div className="min-w-0 flex-1">
					<div className="truncate text-sm font-medium">{d.nombre}</div>
					<div className="text-muted-foreground">
						{t("{n} cartas", { n: d.cartas })}
						{d.proxies ? ` · ${t("con proxies")}` : ""}
					</div>
				</div>
			</div>
			<ToggleGroup
				type="single"
				variant="outline"
				size="sm"
				value={elegido.eleccion}
				onValueChange={(v) => v && onCambio({ ...elegido, eleccion: v as EleccionPasar })}
				className={cn("mt-2 w-full", movil ? "flex-col" : "flex-wrap")}
				aria-label={t("Qué hacer con este deck")}
			>
				{opcionesDe(deck).map((o) => (
					<ToggleGroupItem
						key={o}
						value={o}
						className={cn("h-auto min-h-7 justify-start px-2 py-1 text-xs", movil && "w-full")}
					>
						{nombreOpcion(o)}
					</ToggleGroupItem>
				))}
			</ToggleGroup>
			{deck.conflicto && acepta && (
				<Conflicto
					deck={deck}
					resolucion={elegido.resolucion}
					onCambio={(r) => onCambio({ ...elegido, resolucion: r })}
					onCancelar={() => onCambio({ eleccion: "no", resolucion: null })}
				/>
			)}
		</div>
	);
}

/** «Ya tienes este deck»: cuál es más reciente y qué hacer con el de aquí. */
function Conflicto({
	deck,
	resolucion,
	onCambio,
	onCancelar,
}: {
	deck: DeckRevisado;
	resolucion: ResolucionPasar | null;
	onCambio: (r: ResolucionPasar) => void;
	/** «Cancelar»: este deck no se recibe. */
	onCancelar: () => void;
}) {
	const { t, lang } = useLang();
	const movil = useFormato().tamano === "movil";
	const c = deck.conflicto;
	if (!c) return null;
	const fecha = (ms: number) =>
		new Intl.DateTimeFormat(lang, { dateStyle: "medium", timeStyle: "short" }).format(new Date(ms));
	const quien =
		c.masReciente === "aqui"
			? t("El de aquí es más reciente.")
			: c.masReciente === "llega"
				? t("El que llega es más reciente.")
				: c.masReciente === "igual"
					? t("Es la misma versión.")
					: "";
	return (
		<div className="mt-2 flex flex-col gap-1.5 rounded-lg border border-amber-500/40 bg-amber-500/10 p-2">
			<div className="flex items-start gap-1.5">
				<Icono icon="lucide:triangle-alert" className="mt-0.5 size-3.5 shrink-0 text-amber-500" />
				<div>
					<div className="font-medium">
						{t("Ya tienes este deck: «{nombre}».", { nombre: c.nombre })}
					</div>
					<div className="text-muted-foreground">
						{t("Cambiado por última vez el {fecha}.", { fecha: fecha(c.actualizado) })} {quien}
					</div>
				</div>
			</div>
			<ToggleGroup
				type="single"
				variant="outline"
				size="sm"
				value={resolucion ?? ""}
				onValueChange={(v) => {
					if (v === "cancelar") onCancelar();
					else if (v) onCambio(v as ResolucionPasar);
				}}
				className={cn("w-full", movil ? "flex-col" : "flex-wrap")}
				aria-label={t("Qué hacer con el de aquí")}
			>
				<ToggleGroupItem
					value="reemplazar"
					className={cn("h-auto min-h-7 justify-start px-2 py-1 text-xs", movil && "w-full")}
				>
					{t("Reemplazar el de aquí")}
				</ToggleGroupItem>
				<ToggleGroupItem
					value="guardarLosDos"
					className={cn("h-auto min-h-7 justify-start px-2 py-1 text-xs", movil && "w-full")}
				>
					{t("Guardar los dos")}
				</ToggleGroupItem>
				<ToggleGroupItem
					value="cancelar"
					className={cn("h-auto min-h-7 justify-start px-2 py-1 text-xs", movil && "w-full")}
				>
					{t("Cancelar")}
				</ToggleGroupItem>
			</ToggleGroup>
			{resolucion === "reemplazar" && (
				<p className="text-[11px] leading-snug text-muted-foreground">
					{t(
						"Se queda tu deck con sus etiquetas, favorito, notas, idioma y partidas; toma las cartas, las listas para jugar y la portada del que llega.",
					)}
				</p>
			)}
			{!resolucion && (
				<p className="text-[11px] leading-snug text-muted-foreground">
					{t("Elige una opción, o «No» para dejarlo.")}
				</p>
			)}
		</div>
	);
}
