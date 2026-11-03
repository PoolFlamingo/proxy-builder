import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { DestinoPasar } from "@/bindings/DestinoPasar";
import type { EstadoEmisor } from "@/bindings/EstadoEmisor";
import type { InfoPaquete } from "@/bindings/InfoPaquete";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { DialogCuerpo, DialogFooter } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { useBiblioteca } from "@/features/library/store";
import {
	abrirAjustesDeLaCamara,
	cancelarEscaneo,
	escanearQr,
	escanerDisponible,
} from "@/lib/escaner";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import {
	emisorEnCurso,
	formatoCodigo6,
	pareceCodigoDeDeck,
	pesar,
	porcentaje,
	textoDeError,
} from "./cuentas";
import { useCerca, useEmisor } from "./estado";
import { Resultados } from "./piezas";

/**
 * **«Enviar»**: qué decks (con o sin proxies) y a quién: uno de los equipos de
 * la red local o el código de «Recibir» (pegado o leído del QR). Después, lo
 * que va pasando. Al irse de aquí, lo que estuviera en marcha se cancela.
 */
export function PanelEnviar({
	idsIniciales,
	onCerrar,
}: {
	idsIniciales: string[];
	onCerrar: () => void;
}) {
	const { t, mensaje } = useLang();
	const decks = useBiblioteca((s) => s.decks);
	const [ids, setIds] = useState<string[]>(idsIniciales);
	const [conProxies, setConProxies] = useState(true);
	const [info, setInfo] = useState<Record<string, InfoPaquete>>({});
	const [codigo, setCodigo] = useState("");
	const [errorCodigo, setErrorCodigo] = useState<string | null>(null);
	const [primero, setPrimero] = useState<EstadoEmisor | null>(null);
	const [enviando, setEnviando] = useState(false);
	const [escaneando, setEscaneando] = useState(false);
	const [hayEscaner, setHayEscaner] = useState(false);
	const [sinBusqueda, setSinBusqueda] = useState<string | null>(null);
	const evento = useEmisor();
	const emisor = evento ?? primero;
	const cerca = useCerca();

	useEffect(() => {
		if (!decks)
			void useBiblioteca
				.getState()
				.cargar()
				.catch(() => {});
	}, [decks]);

	useEffect(() => {
		void escanerDisponible().then(setHayEscaner);
	}, []);

	// La lista de equipos de la red local, mientras se está aquí.
	useEffect(() => {
		call("pasar_emisor_olvidar").catch(() => {});
		call("pasar_cerca_buscar").catch((e) => setSinBusqueda(textoDeError(e, t, mensaje)));
		return () => {
			void call("pasar_cerca_parar").catch(() => {});
			void call("pasar_enviar_cancelar").catch(() => {});
			void cancelarEscaneo();
		};
		// Solo al abrir y cerrar la pantalla.
	}, []);

	// Cuánto pesa cada deck elegido y si lleva proxies.
	useEffect(() => {
		let vivo = true;
		for (const id of ids) {
			if (info[id]) continue;
			call("deck_paquete_info", { id })
				.then((i) => vivo && setInfo((m) => ({ ...m, [id]: i })))
				.catch(() => {});
		}
		return () => {
			vivo = false;
		};
	}, [ids, info]);

	const elegidos = useMemo(() => (decks ?? []).filter((d) => ids.includes(d.id)), [decks, ids]);
	const hayProxies = ids.some((id) => info[id]?.proxies);
	const peso = ids.reduce(
		(n, id) => n + ((conProxies ? info[id]?.tamano : info[id]?.tamanoOriginales) ?? 0),
		0,
	);

	const lanzar = useCallback(
		async (destino: DestinoPasar) => {
			setEnviando(true);
			try {
				await call("pasar_enviar", { destino, decks: ids, conProxies: conProxies && hayProxies });
				setPrimero(await call("pasar_emisor_estado"));
			} catch (e) {
				toast.error(textoDeError(e, t, mensaje));
			} finally {
				setEnviando(false);
			}
		},
		[ids, conProxies, hayProxies, t, mensaje],
	);

	const enviarCodigo = useCallback(
		async (texto: string) => {
			setErrorCodigo(null);
			try {
				await call("pasar_codigo_comprobar", { texto });
			} catch (e) {
				setErrorCodigo(textoDeError(e, t, mensaje));
				return;
			}
			await lanzar({ tipo: "codigo", texto });
		},
		[lanzar, t, mensaje],
	);

	const escanear = async () => {
		setEscaneando(true);
		try {
			const r = await escanearQr();
			if (r.tipo === "leido") {
				setCodigo(r.texto);
				if (ids.length > 0) await enviarCodigo(r.texto);
			} else if (r.tipo === "sin-permiso") {
				toast.error(t("Sin permiso para usar la cámara."), {
					action: { label: t("Ajustes"), onClick: () => void abrirAjustesDeLaCamara() },
				});
			}
		} finally {
			setEscaneando(false);
		}
	};

	if (emisor) {
		return (
			<VistaEmisor
				estado={emisor}
				onCerrar={onCerrar}
				onOtro={() => {
					void call("pasar_emisor_olvidar").catch(() => {});
					setPrimero(null);
				}}
			/>
		);
	}

	const alternar = (id: string) =>
		setIds((v) => (v.includes(id) ? v.filter((x) => x !== id) : [...v, id]));
	const puede = ids.length > 0 && !enviando;
	return (
		<>
			<DialogCuerpo>
				<div className="flex flex-col gap-3 p-1">
					<Bloque titulo={t("Qué decks")}>
						{decks === null ? (
							<Icono icon="lucide:loader-2" className="size-4 animate-spin text-muted-foreground" />
						) : decks.length === 0 ? (
							<p className="text-muted-foreground">{t("Todavía no tienes decks.")}</p>
						) : (
							<ScrollArea className="rounded-lg border" viewportClassName="max-h-44">
								<ul className="flex flex-col gap-0.5 p-1">
									{decks.map((d) => (
										<li key={d.id}>
											<label className="flex min-h-7 cursor-pointer items-center gap-2 rounded-md px-1.5 py-1 hover:bg-accent">
												<Checkbox
													checked={ids.includes(d.id)}
													onCheckedChange={() => alternar(d.id)}
												/>
												<span className="min-w-0 flex-1 truncate">{d.name}</span>
												<span className="shrink-0 text-muted-foreground">
													{t("{n} cartas", { n: d.cardCount })}
												</span>
											</label>
										</li>
									))}
								</ul>
							</ScrollArea>
						)}
						{elegidos.length > 0 && (
							<p className="text-muted-foreground">
								{t("{n} decks elegidos", { n: elegidos.length })} · {pesar(peso)}
							</p>
						)}
					</Bloque>

					{hayProxies && (
						<Bloque titulo={t("Con o sin proxies")}>
							<ToggleGroup
								type="single"
								variant="outline"
								size="sm"
								value={conProxies ? "con" : "sin"}
								onValueChange={(v) => v && setConProxies(v === "con")}
								className="w-full flex-wrap"
							>
								<ToggleGroupItem value="con" className="h-auto min-h-7 flex-1 px-2 py-1 text-xs">
									{t("Con proxies")}
								</ToggleGroupItem>
								<ToggleGroupItem value="sin" className="h-auto min-h-7 flex-1 px-2 py-1 text-xs">
									{t("Sin proxies")}
								</ToggleGroupItem>
							</ToggleGroup>
							<p className="text-[11px] leading-snug text-muted-foreground">
								{conProxies
									? t("Van tus imágenes propias y tus diseños.")
									: t(
											"Solo van las cartas originales: las propias vinculadas pasan a su carta y quien recibe baja las imágenes.",
										)}
							</p>
						</Bloque>
					)}

					<Bloque titulo={t("Equipos cerca")}>
						{cerca.length === 0 && sinBusqueda ? (
							<p className="text-muted-foreground">{sinBusqueda}</p>
						) : cerca.length === 0 ? (
							<p className="flex items-center gap-1.5 text-muted-foreground">
								<Icono icon="lucide:loader-2" className="size-3.5 animate-spin" />
								{t("Buscando… El otro equipo tiene que tener abierto «Recibir» en la misma red.")}
							</p>
						) : (
							<ul className="flex flex-col gap-1">
								{cerca.map((c) => (
									<li key={c.id}>
										<Button
											variant="outline"
											className="h-auto min-h-9 w-full justify-start gap-2 px-2.5 py-1.5 text-xs"
											disabled={!puede}
											onClick={() => void lanzar({ tipo: "cerca", id: c.id })}
										>
											<Icono icon="lucide:monitor-smartphone" className="size-4" />
											<span className="min-w-0 flex-1 truncate text-left">
												{c.apodo || t("Equipo sin nombre")}
											</span>
											<span className="text-muted-foreground">{c.huella}</span>
										</Button>
									</li>
								))}
							</ul>
						)}
					</Bloque>

					<Bloque titulo={t("O con el código de «Recibir»")}>
						<div className="flex items-start gap-1.5">
							<Input
								value={codigo}
								onChange={(e) => {
									setCodigo(e.target.value);
									setErrorCodigo(null);
								}}
								placeholder="PBD1-…"
								className="min-w-0 flex-1 font-mono text-xs select-text"
								autoCapitalize="off"
								autoCorrect="off"
								spellCheck={false}
								aria-invalid={!!errorCodigo}
							/>
							{hayEscaner && (
								<Button
									size="icon-sm"
									variant="outline"
									onClick={() => void escanear()}
									disabled={escaneando || ids.length === 0}
									aria-label={t("Escanear un QR")}
								>
									<Icono icon="lucide:scan-line" />
								</Button>
							)}
							<Button
								size="sm"
								onClick={() => void enviarCodigo(codigo)}
								disabled={!puede || !pareceCodigoDeDeck(codigo)}
							>
								<Icono icon="lucide:send" />
								{t("Enviar")}
							</Button>
						</div>
						{errorCodigo && <p className="text-destructive">{errorCodigo}</p>}
						{!errorCodigo && codigo.trim() !== "" && !pareceCodigoDeDeck(codigo) && (
							<p className="text-amber-500">{t("El código de «Recibir» empieza por PBD1-.")}</p>
						)}
					</Bloque>
				</div>
			</DialogCuerpo>
			<DialogFooter>
				<Button size="sm" variant="outline" onClick={onCerrar}>
					{t("Cerrar")}
				</Button>
			</DialogFooter>
		</>
	);
}

function Bloque({ titulo, children }: { titulo: string; children: React.ReactNode }) {
	return (
		<div className="flex flex-col gap-1.5">
			<span className="text-[10px] font-semibold tracking-widest text-muted-foreground uppercase">
				{titulo}
			</span>
			{children}
		</div>
	);
}

/** El envío en marcha, desde que conecta hasta que termina. */
function VistaEmisor({
	estado,
	onCerrar,
	onOtro,
}: {
	estado: EstadoEmisor;
	onCerrar: () => void;
	onOtro: () => void;
}) {
	const { t, mensaje } = useLang();
	const en = emisorEnCurso(estado);
	const quien = estado.remoto?.apodo || estado.remoto?.huella || t("el otro equipo");
	const p = estado.progreso;
	return (
		<>
			<DialogCuerpo>
				<div className="flex flex-col gap-2 p-1">
					{estado.fase === "conectando" && <Espera texto={t("Conectando…")} />}
					{estado.fase === "esperandoAceptacion" && (
						<div className="flex flex-col items-center gap-2 p-3 text-center">
							<p>{t("Esperando a que lo acepten en el otro equipo.")}</p>
							<div className="text-3xl font-semibold tracking-widest tabular-nums select-text">
								{formatoCodigo6(estado.codigo6 ?? "")}
							</div>
							<p className="text-muted-foreground">{t("Tiene que enseñar este mismo código.")}</p>
						</div>
					)}
					{estado.fase === "esperandoRespuesta" && (
						<Espera texto={t("Esperando a que «{nombre}» elija qué quiere…", { nombre: quien })} />
					)}
					{estado.fase === "enviando" && (
						<div className="flex flex-col gap-1.5">
							<div className="flex items-center justify-between">
								<span className="truncate">
									{p
										? t("Enviando «{nombre}» ({i}/{total})", {
												nombre: p.nombre,
												i: Math.min(p.decksHechos + 1, p.decksTotal),
												total: p.decksTotal,
											})
										: t("Enviando…")}
								</span>
								<span className="tabular-nums text-muted-foreground">{porcentaje(p)} %</span>
							</div>
							<Progress value={porcentaje(p)} />
						</div>
					)}
					{estado.fase === "terminado" && <p className="font-medium">{t("Listo.")}</p>}
					{estado.fase === "cancelado" && (
						<p className="font-medium">
							{t(
								"Se ha cancelado. Lo que ya había llegado entero se ha guardado en el otro equipo.",
							)}
						</p>
					)}
					{estado.error && (
						<p className="text-destructive">{textoDeError(estado.error, t, mensaje)}</p>
					)}
					<Resultados resultados={estado.resultados} />
				</div>
			</DialogCuerpo>
			<DialogFooter>
				{en ? (
					<Button size="sm" variant="outline" onClick={() => void call("pasar_enviar_cancelar")}>
						{t("Cancelar")}
					</Button>
				) : (
					<>
						<Button size="sm" variant="outline" onClick={onCerrar}>
							{t("Cerrar")}
						</Button>
						<Button size="sm" onClick={onOtro}>
							{estado.fase === "terminado" ? t("Enviar más") : t("Volver")}
						</Button>
					</>
				)}
			</DialogFooter>
		</>
	);
}

function Espera({ texto }: { texto: string }) {
	return (
		<div className={cn("flex flex-col items-center gap-2 p-4 text-center")}>
			<Icono icon="lucide:loader-2" className="size-5 animate-spin text-muted-foreground" />
			<p>{texto}</p>
		</div>
	);
}
