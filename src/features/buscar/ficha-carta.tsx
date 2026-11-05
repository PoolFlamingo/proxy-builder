import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { AclaracionCarta } from "@/bindings/AclaracionCarta";
import type { CaraFicha } from "@/bindings/CaraFicha";
import type { CardSummary } from "@/bindings/CardSummary";
import type { FichaCarta } from "@/bindings/FichaCarta";
import { CampoNumero } from "@/components/campos";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { RaizScroll } from "@/features/preview/visibilidad";
import { RejillaCartas } from "@/features/variants/rejilla-cartas";
import { traerCarta } from "@/features/variants/traer";
import { insertar, nuevoId } from "@/lib/deck-ops";
import { localeDe } from "@/lib/idiomas";
import { urlBusqueda } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useBusqueda } from "@/stores/busqueda-store";
import { useDeck } from "@/stores/deck-store";
import { FORMATOS, RAREZAS } from "./filtros-panel";
import { TextoConSimbolos } from "./simbolos-mana";

/** La ficha de una carta: imagen, reglas, legalidad, precios e impresiones. */
export function FichaCartaDialog() {
	const id = useBusqueda((s) => s.ficha);
	return (
		<Dialog
			open={id !== null}
			onOpenChange={(abierto) => !abierto && useBusqueda.getState().abrirFicha(null)}
		>
			{id !== null && <Contenido id={id} />}
		</Dialog>
	);
}

function Contenido({ id }: { id: string }) {
	const { t, mensaje } = useLang();
	const [ficha, setFicha] = useState<FichaCarta | null>(null);
	const [fallo, setFallo] = useState(false);
	const [aclaraciones, setAclaraciones] = useState<AclaracionCarta[] | null>(null);
	const [impresiones, setImpresiones] = useState<CardSummary[] | null>(null);
	const [cara, setCara] = useState(0);
	const [copias, setCopias] = useState(1);
	const [anadiendo, setAnadiendo] = useState(false);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
	const abrir = useBusqueda((s) => s.abrirFicha);

	// Al pasar a otra impresión se queda la de antes en pantalla hasta que llega.
	useEffect(() => {
		let vigente = true;
		setFallo(false);
		call("carta_ficha", { id })
			.then((f) => {
				if (!vigente) return;
				setFicha(f);
				setCara(0);
			})
			.catch((e) => {
				if (!vigente) return;
				setFallo(true);
				toast.error(mensaje(e));
			});
		return () => {
			vigente = false;
		};
	}, [id, mensaje]);

	const nombre = ficha?.nombre ?? null;
	const idioma = ficha?.idioma ?? null;
	useEffect(() => {
		if (!nombre || !idioma) return;
		let vigente = true;
		setImpresiones(null);
		// Las de su idioma. Sin ellas (o sin red) la ficha sigue valiendo: es solo una lista.
		call("card_prints", { name: nombre, lang: idioma, highresOnly: false })
			.then((r) => vigente && setImpresiones(r.filter((c) => !c.fuente)))
			.catch(() => vigente && setImpresiones([]));
		return () => {
			vigente = false;
		};
	}, [nombre, idioma]);

	// Las aclaraciones son de la carta, no de la impresión: basta con la primera.
	useEffect(() => {
		if (!nombre) return;
		let vigente = true;
		setAclaraciones(null);
		call("carta_aclaraciones", { id })
			.then((r) => vigente && setAclaraciones(r))
			.catch(() => vigente && setAclaraciones([]));
		return () => {
			vigente = false;
		};
	}, [nombre]);

	const anadir = async () => {
		if (!ficha) return;
		setAnadiendo(true);
		try {
			const base = await traerCarta({ id: ficha.id });
			const nuevas = Array.from({ length: Math.max(1, copias) }, (_, i) =>
				i === 0 ? base : { ...base, id: nuevoId() },
			);
			useDeck.getState().setCartas((cs) => insertar(cs, nuevas));
			toast.success(t("{n} × {carta} añadida", { n: nuevas.length, carta: base.meta.name }));
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setAnadiendo(false);
		}
	};

	const titulo = ficha ? (ficha.nombreImpreso ?? ficha.nombre) : t("Cargando…");
	const caraVista = ficha?.caras[ficha.dobleCara ? cara : 0];
	return (
		<DialogoAdaptable
			className="flex h-[88vh] flex-col sm:max-w-4xl"
			cambiosSinGuardar={false}
			ocupado={anadiendo}
			// Sin foco al abrir: el primer campo es «Copias» y en el móvil sacaría el teclado.
			onOpenAutoFocus={(e) => e.preventDefault()}
		>
			<DialogHeader>
				<DialogTitle className="truncate">{titulo}</DialogTitle>
				<DialogDescription className="truncate">
					{ficha
						? `${ficha.setNombre} · ${ficha.set.toUpperCase()} ${ficha.numero} · ${ficha.idioma.toUpperCase()}`
						: " "}
				</DialogDescription>
			</DialogHeader>
			<DialogCuerpo onViewportChange={setViewport}>
				<RaizScroll.Provider value={viewport}>
					{!ficha ? (
						<div className="flex justify-center py-16 text-muted-foreground">
							{fallo ? (
								<p className="text-xs">{t("No se ha podido cargar la carta.")}</p>
							) : (
								<Icono icon="lucide:loader-2" className="size-5 animate-spin" />
							)}
						</div>
					) : (
						<div className="grid gap-4 p-1 sm:grid-cols-[minmax(0,17rem)_minmax(0,1fr)]">
							<div className="mx-auto flex w-full max-w-64 flex-col gap-2.5 sm:max-w-none">
								<div className="relative aspect-[63/88] overflow-hidden rounded-[4.5%] bg-muted">
									{caraVista?.imagen ? (
										<img
											src={urlBusqueda(caraVista.imagen)}
											alt={caraVista.nombre}
											draggable={false}
											className="size-full object-cover"
										/>
									) : (
										<div className="flex size-full items-center justify-center text-muted-foreground">
											<Icono icon="lucide:image-off" className="size-6" />
										</div>
									)}
								</div>
								{ficha.dobleCara && (
									<Button size="sm" variant="outline" onClick={() => setCara((c) => 1 - c)}>
										<Icono icon="lucide:repeat-2" />
										{t("Ver la otra cara")}
									</Button>
								)}
							</div>
							<div className="flex min-w-0 flex-col gap-4">
								{ficha.caras.map((c, i) => (
									// Las caras de una carta no cambian de sitio.
									<Cara key={i} cara={c} />
								))}
								<Seccion titulo={t("Esta impresión")}>
									<Dato
										nombre={t("Set")}
										valor={`${ficha.setNombre} (${ficha.set.toUpperCase()})`}
									/>
									<Dato nombre={t("Número")} valor={ficha.numero} />
									{ficha.rareza && (
										<Dato nombre={t("Rareza")} valor={nombreRareza(ficha.rareza, t)} />
									)}
									{ficha.fecha && <Dato nombre={t("Fecha de salida")} valor={ficha.fecha} />}
									{caraVista?.artista && <Dato nombre={t("Artista")} valor={caraVista.artista} />}
								</Seccion>
								<Precios ficha={ficha} />
								{ficha.legalidades.length > 0 && (
									<Seccion titulo={t("Legalidad")}>
										<div className="grid grid-cols-2 gap-1">
											{ficha.legalidades.map((l) => (
												<div
													key={l.formato}
													className="flex min-w-0 items-center justify-between gap-1.5 rounded-md border px-2 py-1"
												>
													<span className="truncate">{FORMATOS[l.formato] ?? l.formato}</span>
													<EstadoLegal estado={l.estado} />
												</div>
											))}
										</div>
									</Seccion>
								)}
								{aclaraciones && aclaraciones.length > 0 && (
									<Seccion titulo={t("Aclaraciones de reglas (en inglés)")}>
										<ul className="flex flex-col gap-2">
											{aclaraciones.map((a) => (
												<li key={`${a.fecha}-${a.texto}`} className="flex flex-col gap-0.5">
													<span className="text-[10px] text-muted-foreground">{a.fecha}</span>
													<span className="whitespace-pre-line">
														<TextoConSimbolos texto={a.texto} />
													</span>
												</li>
											))}
										</ul>
									</Seccion>
								)}
							</div>
							{impresiones && impresiones.length > 1 && (
								<div className="flex min-w-0 flex-col gap-1.5 sm:col-span-2">
									<span className="text-[11px] font-semibold text-muted-foreground">
										{t("{n} impresiones", { n: impresiones.length })}
									</span>
									<RejillaCartas
										cartas={impresiones}
										elegidas={new Set([ficha.id])}
										onElegir={(c) => abrir(c.id)}
										columnas="grid-cols-[repeat(3,minmax(0,1fr))] sm:grid-cols-[repeat(auto-fill,minmax(110px,1fr))]"
									/>
								</div>
							)}
						</div>
					)}
				</RaizScroll.Provider>
			</DialogCuerpo>
			<DialogFooter className="items-center">
				<div className="flex items-center justify-between gap-2 sm:contents">
					<span className="text-xs">{t("Copias")}</span>
					<CampoNumero
						valor={copias}
						onChange={(v) => setCopias(Math.max(1, Math.round(v ?? 1)))}
						min={1}
						max={99}
						paso={1}
						ancho="w-16"
					/>
				</div>
				<Button size="sm" onClick={anadir} disabled={!ficha || anadiendo}>
					{anadiendo ? (
						<Icono icon="lucide:loader-2" className="animate-spin" />
					) : (
						<Icono icon="lucide:plus" />
					)}
					{t("Añadir al deck")}
				</Button>
			</DialogFooter>
		</DialogoAdaptable>
	);
}

/** Lo escrito en una cara: lo impreso en su idioma o, si no hay, el Oracle en inglés. */
function Cara({ cara: c }: { cara: CaraFicha }) {
	const texto = c.textoImpreso ?? c.texto;
	const pt = c.fuerza !== null && c.resistencia !== null ? `${c.fuerza}/${c.resistencia}` : null;
	return (
		<div className="flex flex-col gap-1.5 text-xs">
			<div className="flex items-start justify-between gap-2.5">
				<span className="min-w-0 text-sm font-semibold">{c.nombreImpreso ?? c.nombre}</span>
				{c.coste && (
					<span className="shrink-0 text-sm">
						<TextoConSimbolos texto={c.coste} />
					</span>
				)}
			</div>
			{(c.tipoImpreso ?? c.tipo) && (
				<span className="text-muted-foreground">{c.tipoImpreso ?? c.tipo}</span>
			)}
			{texto && (
				<p className="leading-relaxed whitespace-pre-line">
					<TextoConSimbolos texto={texto} />
				</p>
			)}
			{c.ambientacion && (
				<p className="whitespace-pre-line text-muted-foreground italic">{c.ambientacion}</p>
			)}
			{(pt ?? c.lealtad ?? c.defensa) && (
				<span className="self-end rounded-md border px-1.5 py-0.5 font-semibold tabular-nums">
					{pt ?? c.lealtad ?? c.defensa}
				</span>
			)}
		</div>
	);
}

function Precios({ ficha }: { ficha: FichaCarta }) {
	const { t, lang } = useLang();
	const { eur, eurFoil, usd, usdFoil } = ficha.precios;
	const dinero = (valor: string | null, moneda: string) =>
		valor === null
			? "—"
			: new Intl.NumberFormat(localeDe(lang), { style: "currency", currency: moneda }).format(
					Number(valor),
				);
	if (eur === null && eurFoil === null && usd === null && usdFoil === null) return null;
	return (
		<Seccion titulo={t("Precio")}>
			<div className="grid grid-cols-2 gap-x-2.5 gap-y-1">
				<Dato nombre="EUR" valor={dinero(eur, "EUR")} />
				<Dato nombre="EUR foil" valor={dinero(eurFoil, "EUR")} />
				<Dato nombre="USD" valor={dinero(usd, "USD")} />
				<Dato nombre="USD foil" valor={dinero(usdFoil, "USD")} />
			</div>
		</Seccion>
	);
}

function EstadoLegal({ estado }: { estado: string }) {
	const { t } = useLang();
	const [texto, clase] =
		estado === "legal"
			? [t("Legal"), "text-green-700 dark:text-green-400"]
			: estado === "restricted"
				? [t("Restringida"), "text-amber-700 dark:text-amber-400"]
				: estado === "banned"
					? [t("Prohibida"), "text-destructive"]
					: [t("No legal"), "text-muted-foreground"];
	return <span className={cn("shrink-0 text-[10px] font-medium", clase)}>{texto}</span>;
}

function nombreRareza(rareza: string, t: (es: string) => string): string {
	const r = RAREZAS.find((x) => x.id === rareza);
	if (r) return t(r.nombre);
	return rareza === "special" ? t("Especial") : rareza === "bonus" ? t("Bonus") : rareza;
}

function Seccion({ titulo, children }: { titulo: string; children: React.ReactNode }) {
	return (
		<div className="flex flex-col gap-1.5 text-xs">
			<span className="text-[11px] font-semibold text-muted-foreground">{titulo}</span>
			{children}
		</div>
	);
}

function Dato({ nombre, valor }: { nombre: string; valor: string }) {
	return (
		<div className="flex min-w-0 items-baseline justify-between gap-2.5">
			<span className="shrink-0 text-muted-foreground">{nombre}</span>
			<span className="min-w-0 truncate text-right">{valor}</span>
		</div>
	);
}
