import { FilaMenu, HojaMenu } from "@/components/hoja-menu";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
import { cn } from "@/lib/utils";
import { useUi, type Vista } from "@/stores/ui-store";
import { BotonActualizar, PuntoActualizacion, useAvisoAjustes } from "./actualizacion-automatica";

/** Las pantallas que tienen pestaña propia abajo, en este orden. */
export const PESTANAS_MOVIL: { vista: Vista; icono: string; texto: string }[] = [
	{ vista: "deck", icono: "lucide:layers", texto: paraTraducir("Deck") },
	{ vista: "disenar", icono: "lucide:brush", texto: paraTraducir("Diseñar") },
	{ vista: "biblioteca", icono: "lucide:book-open", texto: paraTraducir("Biblioteca") },
	{ vista: "mesa", icono: "game-icons:crossed-swords", texto: paraTraducir("Jugar") },
];

/**
 * **La barra de pestañas de abajo (móvil).** Deck, Diseñar, Biblioteca, Jugar y
 * «Más». Respeta la zona segura de abajo con `env(safe-area-inset-bottom)`
 * (el `#root` ya la lleva como relleno en táctil; por eso aquí no se repite)
 * y se esconde mientras el teclado está abierto (`data-teclado` en `<html>`,
 * `lib/teclado.ts`): quita sitio justo cuando más falta hace.
 *
 * `lado` (el teléfono en horizontal): un carril a la izquierda en vez de una
 * barra abajo. De lado solo hay unos 360 dp de alto y cada fila apilada se nota.
 */
export function BarraInferiorMovil({
	vista,
	onCambiar,
	onMas,
	masAbierto,
	lado = false,
}: {
	vista: Vista;
	onCambiar: (v: Vista) => void;
	onMas: () => void;
	masAbierto: boolean;
	lado?: boolean;
}) {
	const { t } = useLang();
	const avisoActualizacion = useAvisoAjustes();
	return (
		<nav
			aria-label={t("Pantallas")}
			data-tutorial="pestanas"
			className={cn(
				"flex shrink-0 bg-background [[data-teclado]_&]:hidden",
				lado ? "order-first w-16 flex-col border-r" : "border-t",
			)}
		>
			{PESTANAS_MOVIL.map((p) => {
				const activa = vista === p.vista && !masAbierto;
				return (
					<button
						key={p.vista}
						type="button"
						aria-current={activa ? "page" : undefined}
						onClick={() => onCambiar(p.vista)}
						className={cn(
							"flex min-w-0 flex-1 flex-col items-center justify-center gap-0.5 px-1 text-[11px] font-medium",
							lado ? "min-h-0" : "min-h-14",
							activa ? "text-primary" : "text-muted-foreground",
						)}
					>
						<Icono icon={p.icono} className="size-5" />
						<span className="max-w-full truncate">{t(p.texto)}</span>
					</button>
				);
			})}
			<button
				type="button"
				aria-haspopup="dialog"
				aria-expanded={masAbierto}
				onClick={onMas}
				data-tutorial="ajustes"
				className={cn(
					"relative flex min-w-0 flex-1 flex-col items-center justify-center gap-0.5 px-1 text-[11px] font-medium",
					lado ? "min-h-0" : "min-h-14",
					masAbierto ? "text-primary" : "text-muted-foreground",
				)}
				aria-label={avisoActualizacion ? `${t("Más")}: ${avisoActualizacion}` : t("Más")}
			>
				<span className="relative">
					<Icono icon="lucide:menu" className="size-5" />
					<PuntoActualizacion />
				</span>
				<span className="max-w-full truncate">{t("Más")}</span>
			</button>
		</nav>
	);
}

/**
 * **El «Más» del móvil**: una hoja con Ajustes, el tutorial de la pantalla en
 * que se está y la actualización (si hay una bajada, su botón de reiniciar).
 */
export function HojaMas({
	abierta,
	onCerrar,
	onAjustes,
	vista,
}: {
	abierta: boolean;
	onCerrar: () => void;
	onAjustes: () => void;
	vista: Vista;
}) {
	const { t } = useLang();
	const aviso = useAvisoAjustes();
	return (
		<HojaMenu abierta={abierta} onCerrar={onCerrar} titulo={t("Más")} sinRelleno>
			<FilaMenu
				icono="lucide:settings"
				texto={t("Ajustes")}
				onClick={onAjustes}
				marca={<PuntoActualizacion />}
			/>
			<FilaMenu
				icono="lucide:circle-help"
				texto={t("Tutorial de esta pantalla")}
				onClick={() => useUi.getState().abrirTutorial(vista)}
			/>
			{aviso && <p className="px-4 py-2 text-xs text-muted-foreground">{aviso}</p>}
			<div className="flex px-4 py-2 empty:hidden">
				<BotonActualizar />
			</div>
		</HojaMenu>
	);
}

/** Las tres pestañas dentro de Deck en el móvil. */
export type SubpestanaDeck = "lista" | "hojas" | "exportar";

const SUBPESTANAS: { id: SubpestanaDeck; texto: string; icono: string }[] = [
	{ id: "lista", texto: paraTraducir("Lista"), icono: "lucide:list" },
	{ id: "hojas", texto: paraTraducir("Hojas"), icono: "lucide:files" },
	{ id: "exportar", texto: paraTraducir("Exportar"), icono: "lucide:printer" },
];

/** Lista, Hojas y Exportar, bajo la barra de arriba, en la pantalla Deck del móvil. */
export function PestanasDeck({
	valor,
	onCambiar,
	enCabecera = false,
}: {
	valor: SubpestanaDeck;
	onCambiar: (v: SubpestanaDeck) => void;
	/** De lado: dentro de la cabecera, en vez de en una fila más. */
	enCabecera?: boolean;
}) {
	const { t } = useLang();
	return (
		<div
			role="tablist"
			aria-label={t("Deck")}
			className={cn(
				"flex shrink-0 bg-background",
				enCabecera ? "h-full [&>button]:px-3" : "border-b",
			)}
		>
			{SUBPESTANAS.map((p) => (
				<button
					key={p.id}
					type="button"
					role="tab"
					aria-selected={valor === p.id}
					onClick={() => onCambiar(p.id)}
					className={cn(
						"flex min-h-11 flex-1 items-center justify-center gap-1.5 border-b-2 text-xs font-medium",
						valor === p.id
							? "border-primary text-primary"
							: "border-transparent text-muted-foreground",
					)}
				>
					<Icono icon={p.icono} className="size-4" />
					{t(p.texto)}
				</button>
			))}
		</div>
	);
}
