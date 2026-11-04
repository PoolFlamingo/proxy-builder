import type { ReactNode } from "react";
import type { Formato } from "@/bindings/Formato";
import type { ListasDeDeck } from "@/bindings/ListasDeDeck";
import { Fila } from "@/components/campos";
import { HojaMenu } from "@/components/hoja-menu";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { MAX_NOMBRE } from "./nombre";

/** El valor del selector de listas para jugar con el deck entero (Radix no admite «»). */
export const TODO_EL_DECK = "*";

/**
 * **El panel de empezar de «Jugar» en el móvil**: lo justo para empezar,
 * siempre a la vista (abajo de pie, a la derecha de lado) para que los decks
 * se queden con el resto de la pantalla. El formato y la lista van aquí; lo
 * demás (nombre, vidas, mulligan, ayudas, sparring, tapete) en la hoja
 * «Opciones», que es la misma lista de campos que en el escritorio.
 */
export function PanelEmpezarMovil({
	lado,
	extra,
	guardada,
	nombre,
	onNombre,
	formato,
	formatos,
	onFormato,
	listas,
	listaId,
	onLista,
	pideLista,
	sinLista,
	onNuevaLista,
	onEditarLista,
	opciones,
	abiertas,
	onAbiertas,
	botones,
}: {
	/** De lado (el móvil en horizontal): una columna a la derecha con scroll propio. */
	lado: boolean;
	/** Lo que va arriba del todo (volver a una partida en red). */
	extra?: ReactNode;
	guardada: { texto: string; seguir: () => void; descartar: () => void; ocupado: boolean } | null;
	nombre: string;
	onNombre: (v: string) => void;
	formato: Formato;
	formatos: { valor: Formato; etiqueta: string }[];
	onFormato: (f: Formato) => void;
	/** Las listas del deck elegido (`null`: sin deck o aún no han llegado). */
	listas: ListasDeDeck | null;
	listaId: string | null;
	onLista: (id: string | null) => void;
	pideLista: boolean;
	sinLista: boolean;
	onNuevaLista: () => void;
	onEditarLista: () => void;
	/** Los campos de la hoja «Opciones». */
	opciones: ReactNode;
	abiertas: boolean;
	onAbiertas: (v: boolean) => void;
	/** Los botones de empezar. */
	botones: ReactNode;
}) {
	const { t } = useLang();
	const sinNingunaLista = pideLista && (listas?.listas.length ?? 0) === 0;
	const campos = (
		<div className="flex flex-col gap-2">
			{guardada && (
				<div className="flex min-w-0 items-center gap-2 rounded-lg border border-primary/40 bg-primary/5 py-1 pr-1 pl-2.5">
					<div className="flex min-w-0 flex-1 flex-col">
						<span className="truncate text-xs font-semibold">{t("Partida a medias")}</span>
						<span className="truncate text-[11px] text-muted-foreground">{guardada.texto}</span>
					</div>
					<Button size="sm" disabled={guardada.ocupado} onClick={guardada.seguir}>
						<Icono icon="lucide:play" />
						{t("Seguir")}
					</Button>
					<Button
						size="icon-sm"
						variant="ghost"
						aria-label={t("Descartar")}
						onClick={guardada.descartar}
					>
						<Icono icon="lucide:trash-2" />
					</Button>
				</div>
			)}
			<div
				className={cn("flex min-w-0 items-center gap-1.5", lado && "flex-wrap")}
				data-tutorial="mesa-lista"
			>
				<div className="flex min-w-0 flex-[4] basis-0">
					<Desplegable
						valor={formato}
						onChange={(v) => onFormato(v as Formato)}
						opciones={formatos}
						etiqueta={t("Formato")}
					/>
				</div>
				{/* De lado va en su propia fila: hay alto de sobra y el nombre se lee entero. */}
				<div className={cn("flex min-w-0 flex-[5] basis-0", lado && "order-last basis-full")}>
					{!listas ? (
						<Desplegable valor="" onChange={() => {}} opciones={[]} etiqueta="" desactivado />
					) : sinNingunaLista ? (
						<Button variant="outline" className="h-10 w-full min-w-0 flex-1" onClick={onNuevaLista}>
							<Icono icon="lucide:plus" />
							<span className="truncate">{t("Nueva lista…")}</span>
						</Button>
					) : (
						<Desplegable
							valor={listaId ?? TODO_EL_DECK}
							onChange={(v) => onLista(v === TODO_EL_DECK ? null : v)}
							etiqueta={t("Lista para jugar")}
							// Crear y editar listas, al final del propio desplegable: sin otro
							// botón al lado, la lista elegida cabe entera.
							acciones={[
								{
									valor: ":nueva",
									etiqueta: t("Nueva lista…"),
									icono: "lucide:plus",
									hacer: onNuevaLista,
								},
								...(listaId
									? [
											{
												valor: ":editar",
												etiqueta: t("Editar…"),
												icono: "lucide:pencil",
												hacer: onEditarLista,
											},
										]
									: []),
							]}
							opciones={[
								...(pideLista
									? []
									: [
											{
												valor: TODO_EL_DECK,
												etiqueta: t("Todo el deck ({n})", { n: listas.cuentan }),
											},
										]),
								...listas.listas.map((l) => ({ valor: l.id, etiqueta: l.nombre })),
							]}
						/>
					)}
				</div>
				<Button
					size="icon-sm"
					variant="outline"
					onClick={() => onAbiertas(true)}
					aria-label={t("Opciones")}
					className="size-10 shrink-0"
				>
					<Icono icon="lucide:sliders-horizontal" />
				</Button>
			</div>
			{sinLista && listas && (
				<p className="flex items-start gap-1.5 rounded-md bg-amber-500/10 p-1.5 text-[11px] text-amber-600 dark:text-amber-300">
					<Icono icon="lucide:list-checks" className="mt-0.5 size-3.5 shrink-0" />
					{t(
						"Este deck tiene {n} cartas y en Commander se juega con 100: haz una lista con el comandante y 99 más.",
						{ n: listas.cuentan },
					)}
				</p>
			)}
		</div>
	);
	return (
		<aside
			className={cn(
				"flex min-h-0 min-w-0 shrink-0 flex-col gap-1.5 bg-background p-2",
				lado ? "w-80 border-l" : "w-full border-t",
			)}
		>
			{extra}
			{lado ? (
				// De lado no hay alto para todo: los campos se desplazan y los botones no.
				<ScrollArea className="-mx-2 min-h-0 flex-1">
					<div className="px-2">{campos}</div>
				</ScrollArea>
			) : (
				campos
			)}
			{botones}
			<HojaMenu
				abierta={abiertas}
				onCerrar={() => onAbiertas(false)}
				titulo={t("Opciones de la partida")}
			>
				<Fila etiqueta={t("Tu nombre")}>
					<Input
						value={nombre}
						onChange={(e) => onNombre(e.target.value)}
						placeholder={t("Jugador")}
						maxLength={MAX_NOMBRE}
						className="h-10 w-44 text-sm"
					/>
				</Fila>
				{opciones}
			</HojaMenu>
		</aside>
	);
}

/** Un desplegable a todo el ancho de su casilla y de la altura de un dedo. */
function Desplegable({
	valor,
	onChange,
	opciones,
	etiqueta,
	desactivado,
	acciones = [],
}: {
	valor: string;
	onChange: (v: string) => void;
	opciones: { valor: string; etiqueta: string }[];
	etiqueta: string;
	desactivado?: boolean;
	/** Filas que hacen algo en vez de elegirse (el valor no cambia). */
	acciones?: { valor: string; etiqueta: string; icono: string; hacer: () => void }[];
}) {
	const elegir = (v: string) => {
		const accion = acciones.find((a) => a.valor === v);
		if (accion) accion.hacer();
		else onChange(v);
	};
	return (
		<Select value={valor} onValueChange={elegir} disabled={desactivado}>
			<SelectTrigger
				className="h-10 w-full min-w-0 px-2.5 text-sm [&_[data-slot=select-value]]:block [&_[data-slot=select-value]]:truncate"
				aria-label={etiqueta}
			>
				<SelectValue />
			</SelectTrigger>
			<SelectContent>
				{opciones.map((o) => (
					<SelectItem key={o.valor} value={o.valor} className="text-sm">
						{o.etiqueta}
					</SelectItem>
				))}
				{acciones.length > 0 && <SelectSeparator />}
				{acciones.map((a) => (
					<SelectItem key={a.valor} value={a.valor} className="text-sm">
						<Icono icon={a.icono} />
						{a.etiqueta}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}
