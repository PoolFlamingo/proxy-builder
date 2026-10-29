import { Clock, Minus, Plus, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { FilaPt } from "@/bindings/FilaPt";
import type { IdCarta } from "@/bindings/IdCarta";
import type { ReglasCarta } from "@/bindings/ReglasCarta";
import { CampoNumero, FilaInterruptor } from "@/components/campos";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { localeDe } from "@/lib/idiomas";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { hacer } from "../acciones";
import { ATAJOS, bonita } from "../atajos";
import { COLORES_ETIQUETA, CONTADORES_CARTA, CONTADORES_OTROS, CONTADORES_PT } from "../colores";
import { CONTADORES_MANA, conTemporal, esTemporal, nombreContador } from "../contadores";
import { caraVisible, ImagenCarta } from "../imagen-carta";
import { type DialogoMesa, usePantalla } from "../pantalla";

const cerrar = () => usePantalla.getState().abrir(null);

/** Pide un número (moler N, robar N, daño…). */
export function NumeroDialog({ d }: { d: Extract<DialogoMesa, { tipo: "numero" }> }) {
	const { t } = useLang();
	const [texto, setTexto] = useState(String(d.inicial));
	const n = Number.parseInt(texto, 10);
	const valido = Number.isFinite(n) && n !== 0 && Math.abs(n) <= 1000;
	const ok = () => {
		if (!valido) return;
		cerrar();
		d.hacer(n);
	};
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-xs" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{d.titulo}</DialogTitle>
				</DialogHeader>
				<Input
					autoFocus
					inputMode="numeric"
					value={texto}
					onChange={(e) => setTexto(e.target.value)}
					onKeyDown={(e) => e.key === "Enter" && ok()}
					onFocus={(e) => e.currentTarget.select()}
					className="text-center text-lg"
				/>
				<DialogFooter>
					<Button variant="outline" onClick={cerrar}>
						{t("Cancelar")}
					</Button>
					<Button onClick={ok} disabled={!valido}>
						{t("Aceptar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

/** Pide un texto (habilidad a la pila). */
export function TextoDialog({ d }: { d: Extract<DialogoMesa, { tipo: "texto" }> }) {
	const { t } = useLang();
	const [texto, setTexto] = useState("");
	const ok = () => {
		if (!texto.trim()) return;
		cerrar();
		d.hacer(texto.trim());
	};
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle>{d.titulo}</DialogTitle>
				</DialogHeader>
				<Input
					autoFocus
					value={texto}
					maxLength={d.max}
					onChange={(e) => setTexto(e.target.value)}
					onKeyDown={(e) => e.key === "Enter" && ok()}
					className="select-text"
				/>
				<DialogFooter>
					<Button variant="outline" onClick={cerrar}>
						{t("Cancelar")}
					</Button>
					<Button onClick={ok} disabled={!texto.trim()}>
						{t("Aceptar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

/** Añadir un contador: el tipo (de la lista o escrito) y cuántos. */
export function ContadorDialog({
	cartas,
	temporal: temporalInicial = false,
}: {
	cartas: IdCarta[];
	temporal?: boolean;
}) {
	const { t, lang } = useLang();
	// Lo que se añade (`tipo`) y lo que se escribe para buscar (`texto`) son cosas
	// distintas: el buscador empieza vacío y enseña todo el catálogo; escribir
	// ahí también deja un tipo propio (vacío: hay que elegir uno), y pulsar un
	// botón elige ese tipo sin tocar el buscador.
	const [tipo, setTipo] = useState("+1/+1");
	const [texto, setTexto] = useState("");
	const [n, setN] = useState(1);
	const [temporal, setTemporal] = useState(temporalInicial);
	const ok = () => {
		cerrar();
		void hacer({
			que: "contador",
			cartas,
			tipo: conTemporal(tipo.trim(), temporal),
			delta: n,
		});
	};
	const filtro = texto.trim().toLowerCase();
	const secciones = [
		{ titulo: t("Habituales"), items: CONTADORES_CARTA, ordenar: false },
		{ titulo: t("Maná"), items: CONTADORES_MANA, ordenar: false },
		{
			titulo: t("Fuerza y resistencia"),
			items: CONTADORES_PT.map((c) => ({ tipo: c, nombre: c })),
			ordenar: false,
		},
		{ titulo: t("Otros contadores"), items: CONTADORES_OTROS, ordenar: true },
	]
		.map((sec) => ({
			...sec,
			items: sec.items
				.map((c) => ({ tipo: c.tipo, nombre: t(c.nombre) }))
				.sort((a, b) => (sec.ordenar ? a.nombre.localeCompare(b.nombre, localeDe(lang)) : 0))
				.filter(
					(c) =>
						filtro === "" ||
						c.tipo.toLowerCase().includes(filtro) ||
						c.nombre.toLowerCase().includes(filtro),
				),
		}))
		.filter((sec) => sec.items.length > 0);
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-md" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Añadir contador")}</DialogTitle>
				</DialogHeader>
				<div className="flex gap-2.5">
					<Input
						value={texto}
						onChange={(e) => {
							setTexto(e.target.value);
							setTipo(e.target.value.trim());
						}}
						maxLength={29}
						placeholder={t("Otro tipo…")}
						className="h-8 flex-1 text-xs"
					/>
					<CampoNumero
						valor={n}
						onChange={(v) => setN(v ?? 0)}
						paso={1}
						ariaLabel={t("Cantidad")}
					/>
				</div>
				<FilaInterruptor
					etiqueta={t("Temporal")}
					ayuda={t("Se pierde al terminar el turno.")}
					valor={temporal}
					onChange={setTemporal}
				/>
				<ScrollArea viewportClassName="max-h-[40vh]">
					<div className="flex flex-col gap-2.5 p-1">
						{secciones.map((sec) => (
							<div key={sec.titulo} className="flex flex-col gap-1.5">
								<span className="text-[10px] font-semibold tracking-wide text-muted-foreground uppercase">
									{sec.titulo}
								</span>
								<div className="flex flex-wrap gap-1.5">
									{sec.items.map((c) => (
										<Button
											key={c.tipo}
											size="sm"
											variant={tipo.trim() === c.tipo ? "default" : "outline"}
											onClick={() => setTipo(c.tipo)}
										>
											{c.nombre}
										</Button>
									))}
								</div>
							</div>
						))}
					</div>
				</ScrollArea>
				<DialogFooter>
					<Button variant="outline" onClick={cerrar}>
						{t("Cancelar")}
					</Button>
					<Button onClick={ok} disabled={!tipo.trim() || n === 0}>
						{t("Añadir")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

type FilaDeContador = { tipo: string; n: number; clave: boolean };

/**
 * **Editar los contadores de una carta** sin quitarlos y volverlos a poner:
 * cambiar la cantidad (con los botones o escribiéndola), quitarlos, y pasarlos
 * de permanentes a temporales (se pierden al terminar el turno) y al revés.
 * Salen en dos grupos; los de palabra clave van sin cantidad.
 */
export function EditarContadoresDialog({ carta }: { carta: IdCarta }) {
	const { t } = useLang();
	const c = useMesa((s) => s.cartas[carta]);
	useEffect(() => {
		if (!c) cerrar();
	}, [c]);
	if (!c) return null;
	const filas: FilaDeContador[] = [
		...Object.entries(c.contadores).map(([tipo, n]) => ({ tipo, n, clave: false })),
		...c.palabrasClave.map((tipo) => ({ tipo, n: 1, clave: true })),
	];
	const grupos = [
		{ titulo: t("Contadores permanentes"), temporal: false },
		{ titulo: t("Contadores temporales"), temporal: true },
	]
		.map((g) => ({ ...g, filas: filas.filter((f) => esTemporal(f.tipo) === g.temporal) }))
		.filter((g) => g.filas.length > 0);
	const sumar = (tipo: string, delta: number) =>
		hacer({ que: "contador", cartas: [carta], tipo, delta });
	const fijar = (tipo: string, valor: number) =>
		hacer({ que: "fijarContador", carta, tipo, valor });
	const clave = (palabra: string, si: boolean) =>
		hacer({ que: "contadorClave", cartas: [carta], palabra, si });
	// Se pone el nuevo antes de quitar el viejo: si algo falla, no se pierde nada.
	const alternar = async (f: FilaDeContador) => {
		const nuevo = conTemporal(f.tipo, !esTemporal(f.tipo));
		if (f.clave) {
			if (await clave(nuevo, true)) await clave(f.tipo, false);
		} else if (await sumar(nuevo, f.n)) {
			await fijar(f.tipo, 0);
		}
	};
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-md" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Editar contadores")}</DialogTitle>
					<DialogDescription>{c.origen?.nombre}</DialogDescription>
				</DialogHeader>
				<ScrollArea viewportClassName="max-h-[55vh]">
					<div className="flex flex-col gap-3 p-1">
						{grupos.length === 0 && (
							<p className="text-xs text-muted-foreground">
								{t("Esta carta no tiene contadores.")}
							</p>
						)}
						{grupos.map((g) => (
							<div key={g.titulo} className="flex flex-col gap-1.5">
								<span className="flex items-center gap-1 text-[10px] font-semibold tracking-wide text-muted-foreground uppercase">
									{g.temporal && <Clock className="size-3" />}
									{g.titulo}
								</span>
								{g.filas.map((f) => (
									<div key={`${f.clave}${f.tipo}`} className="flex items-center gap-1.5">
										<span className="min-w-0 flex-1 truncate text-xs">
											{nombreContador(f.tipo, t)}
										</span>
										{!f.clave && (
											<>
												<Button
													size="icon-sm"
													variant="outline"
													aria-label={t("Quitar uno")}
													onClick={() => void sumar(f.tipo, -1)}
												>
													<Minus />
												</Button>
												<CampoNumero
													valor={f.n}
													onChange={(v) => void fijar(f.tipo, v ?? 0)}
													min={-1_000_000}
													max={1_000_000}
													paso={1}
													ancho="w-16"
													ariaLabel={t("Cantidad")}
												/>
												<Button
													size="icon-sm"
													variant="outline"
													aria-label={t("Añadir uno")}
													onClick={() => void sumar(f.tipo, 1)}
												>
													<Plus />
												</Button>
											</>
										)}
										<WithTooltip
											content={
												g.temporal
													? t("Hacerlo permanente")
													: t("Hacerlo temporal (se pierde al terminar el turno)")
											}
										>
											<Button
												size="icon-sm"
												variant={g.temporal ? "default" : "outline"}
												aria-label={g.temporal ? t("Hacerlo permanente") : t("Hacerlo temporal")}
												onClick={() => void alternar(f)}
											>
												<Clock />
											</Button>
										</WithTooltip>
										<Button
											size="icon-sm"
											variant="outline"
											aria-label={t("Quitar")}
											onClick={() => void (f.clave ? clave(f.tipo, false) : fijar(f.tipo, 0))}
										>
											<Trash2 />
										</Button>
									</div>
								))}
							</div>
						))}
					</div>
				</ScrollArea>
				<DialogFooter>
					<Button onClick={cerrar}>{t("Cerrar")}</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

/** Las filas de fuerza y resistencia de una carta (`capturas/playgroup`, §2.2). */
export function PtDialog({ carta }: { carta: IdCarta }) {
	const { t } = useLang();
	const c = useMesa((s) => s.cartas[carta]);
	if (!c) return null;
	const fila = (nombre: string, f: FilaPt, valor: [number, number] | null, nulo: boolean) => (
		<FilaPtEditable
			key={f}
			nombre={nombre}
			valor={valor}
			nulo={nulo}
			onCambio={(v) => void hacer({ que: "pt", carta, fila: f, valor: v })}
		/>
	);
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-sm" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Fuerza y resistencia")}</DialogTitle>
					<DialogDescription>
						{t("Impresa: {f}/{r}", {
							f: c.origen?.info.fuerza ?? "–",
							r: c.origen?.info.resistencia ?? "–",
						})}
					</DialogDescription>
				</DialogHeader>
				{fila(t("Otros bonos"), "otros", c.pt.otros, false)}
				{fila(t("Hasta el final del turno"), "hastaFinDeTurno", c.pt.hastaFinDeTurno, false)}
				{fila(t("Fija (tapa todo lo demás)"), "fijo", c.pt.fijo, true)}
			</DialogContent>
		</Dialog>
	);
}

function FilaPtEditable({
	nombre,
	valor,
	nulo,
	onCambio,
}: {
	nombre: string;
	valor: [number, number] | null;
	nulo: boolean;
	onCambio: (v: [number, number] | null) => void;
}) {
	const { t } = useLang();
	const [f, setF] = useState(String(valor?.[0] ?? 0));
	const [r, setR] = useState(String(valor?.[1] ?? 0));
	useEffect(() => {
		setF(String(valor?.[0] ?? 0));
		setR(String(valor?.[1] ?? 0));
	}, [valor]);
	const aplicar = () => onCambio([Number.parseInt(f, 10) || 0, Number.parseInt(r, 10) || 0]);
	return (
		<div className="flex items-center gap-2.5 text-xs">
			<span className="flex-1">{nombre}</span>
			<Input
				value={f}
				onChange={(e) => setF(e.target.value)}
				onBlur={aplicar}
				className="h-7 w-14 text-center"
			/>
			<span>/</span>
			<Input
				value={r}
				onChange={(e) => setR(e.target.value)}
				onBlur={aplicar}
				className="h-7 w-14 text-center"
			/>
			{nulo && (
				<Button size="sm" variant="ghost" onClick={() => onCambio(null)} disabled={valor === null}>
					{t("Quitar")}
				</Button>
			)}
		</div>
	);
}

export function EtiquetaDialog({ carta }: { carta: IdCarta }) {
	const { t } = useLang();
	const c = useMesa((s) => s.cartas[carta]);
	const [texto, setTexto] = useState("");
	const [color, setColor] = useState(0);
	if (!c) return null;
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-sm" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Etiquetas")}</DialogTitle>
					<DialogDescription>{t("Hasta 4 por carta, de 20 letras como mucho.")}</DialogDescription>
				</DialogHeader>
				<div className="flex flex-wrap gap-1.5">
					{c.etiquetas.map((e) => (
						<button
							key={e.texto}
							type="button"
							onClick={() => void hacer({ que: "etiqueta", carta, etiqueta: e, poner: false })}
							className="rounded px-1.5 py-0.5 text-[11px] text-white"
							style={{ background: COLORES_ETIQUETA[e.color] }}
						>
							{e.texto} ✕
						</button>
					))}
				</div>
				<div className="flex flex-wrap gap-1.5 p-1">
					{COLORES_ETIQUETA.map((col, i) => (
						<button
							key={col}
							type="button"
							aria-label={col}
							onClick={() => setColor(i)}
							className={cn("size-5 rounded-full", color === i && "ring-2 ring-foreground")}
							style={{ background: col }}
						/>
					))}
				</div>
				<div className="flex gap-2.5">
					<Input
						autoFocus
						value={texto}
						maxLength={20}
						onChange={(e) => setTexto(e.target.value)}
						onKeyDown={(e) => {
							if (e.key === "Enter" && texto.trim()) {
								void hacer({
									que: "etiqueta",
									carta,
									etiqueta: { texto: texto.trim(), color },
									poner: true,
								});
								setTexto("");
							}
						}}
						className="h-8 text-xs"
					/>
					<Button
						size="sm"
						disabled={!texto.trim() || c.etiquetas.length >= 4}
						onClick={() => {
							void hacer({
								que: "etiqueta",
								carta,
								etiqueta: { texto: texto.trim(), color },
								poner: true,
							});
							setTexto("");
						}}
					>
						{t("Añadir")}
					</Button>
				</div>
			</DialogContent>
		</Dialog>
	);
}

/** «Inspeccionar»: la carta en grande y su texto de reglas. */
export function InspeccionDialog() {
	const { t, mensaje } = useLang();
	const id = useMesa((s) => s.inspeccion);
	const c = useMesa((s) => (s.inspeccion !== null ? s.cartas[s.inspeccion] : undefined));
	const [reglas, setReglas] = useState<ReglasCarta | null>(null);
	useEffect(() => {
		setReglas(null);
		if (id === null) return;
		call("mesa_info_carta", { carta: id })
			.then(setReglas)
			.catch((e) => toast.error(mensaje(e)));
	}, [id, mensaje]);
	if (id === null || !c) return null;
	return (
		<Dialog open onOpenChange={(o) => !o && useMesa.getState().inspeccionar(null)}>
			<DialogContent className="sm:max-w-3xl">
				<DialogHeader>
					<DialogTitle>{c.origen?.nombre ?? t("Carta boca abajo")}</DialogTitle>
				</DialogHeader>
				<div className="flex gap-2.5">
					<div className="flex shrink-0 gap-2.5">
						{(c.origen?.caras ?? [caraVisible(c)]).map((cara, i) => (
							<div key={i} className="h-[350px] w-[250px] overflow-hidden rounded-[4.5%]">
								<ImagenCarta imagen={cara} ancho={488} nombre={c.origen?.nombre} />
							</div>
						))}
					</div>
					<ScrollArea className="min-h-0 flex-1" viewportClassName="max-h-[350px]">
						<div className="flex flex-col gap-2.5 p-1 text-xs select-text">
							{reglas?.caras.map((f) => (
								<div key={f.nombre} className="flex flex-col gap-1">
									<div className="flex justify-between font-semibold">
										<span>{f.nombre}</span>
										<span className="text-muted-foreground">{f.coste}</span>
									</div>
									<span className="text-muted-foreground">{f.tipo}</span>
									{f.texto && <p className="whitespace-pre-wrap">{f.texto}</p>}
									{(f.fuerza || f.lealtad) && (
										<span className="font-semibold">
											{f.lealtad ?? `${f.fuerza}/${f.resistencia}`}
										</span>
									)}
								</div>
							))}
						</div>
					</ScrollArea>
				</div>
			</DialogContent>
		</Dialog>
	);
}

const CONTEXTOS = [
	["carta", paraTraducir("Con el ratón sobre una carta (o elegidas)")],
	["biblioteca", paraTraducir("Con el ratón sobre la biblioteca")],
	["mesa", paraTraducir("En toda la mesa")],
] as const;

export function AtajosDialog() {
	const { t } = useLang();
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-2xl">
				<DialogHeader>
					<DialogTitle>{t("Atajos de teclado")}</DialogTitle>
					<DialogDescription>
						{t(
							"Sobre la biblioteca, del 1 al 9 roba esas cartas. Doble clic en una carta del campo la gira.",
						)}{" "}
						{t("En red, Alt + clic en una carta o en un jugador avisa a todos con un aro.")}
					</DialogDescription>
				</DialogHeader>
				<ScrollArea viewportClassName="max-h-[60vh]">
					<div className="grid grid-cols-3 gap-4 p-1 text-xs">
						{CONTEXTOS.map(([c, titulo]) => (
							<div key={c} className="flex flex-col gap-1">
								<h4 className="font-semibold">{t(titulo)}</h4>
								{ATAJOS.filter((a) => a.contexto === c).map((a) => (
									<div key={a.tecla} className="flex justify-between gap-2">
										<span>{t(a.nombre)}</span>
										<kbd className="rounded border bg-muted px-1 font-mono text-[10px]">
											{bonita(a.tecla)}
										</kbd>
									</div>
								))}
							</div>
						))}
					</div>
				</ScrollArea>
			</DialogContent>
		</Dialog>
	);
}

/** Lo que se recuerda de Ajustes > Mesa para no pintar la vida del fantasma. */
export function useVerFantasma() {
	return useUi((s) => s.mesa.fantasma);
}
