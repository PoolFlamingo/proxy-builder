import { type RefObject, useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { AppError } from "@/bindings/AppError";
import type { EstadoReceptor } from "@/bindings/EstadoReceptor";
import type { ResultadoPasar } from "@/bindings/ResultadoPasar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { DialogCuerpo, DialogFooter } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { useFormato } from "@/lib/formato";
import { call } from "@/lib/ipc";
import { useEsMovil } from "@/lib/plataforma";
import { cn } from "@/lib/utils";
import { abrirDeck, type Devolver, prestarDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import {
	aDecisiones,
	cuentaAtras,
	type Elegido,
	eleccionesIniciales,
	formatoCifras,
	pesar,
	porcentaje,
	puedeResponder,
	quedaAlgo,
	reemplazados,
	textoDeError,
} from "./cuentas";
import { useReceptor } from "./estado";
import { useBajarImagenes } from "./imagenes";
import { OfertaDecks } from "./oferta";
import { ProgresoImagenes, Resultados } from "./piezas";

/** Un SVG de Rust, como imagen (nada de pintar HTML ajeno). */
export function urlSvg(svg: string): string {
	return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/**
 * **«Recibir»**: abre el modo de espera (Rust: `pasar_recibir_abrir`), enseña
 * el QR y el código, deja aceptar a quien llega por la red local, enseña la
 * oferta para elegir y sigue lo que se recibe. Al irse de aquí, el modo se
 * cierra (y corta lo que hubiera en curso).
 */
export function PanelRecibir({ onCerrar }: { onCerrar: () => void }) {
	const { t, mensaje } = useLang();
	const [fallo, setFallo] = useState<string | null>(null);
	// Lo que contesta `pasar_recibir_abrir` vale hasta que llega el primer
	// evento (que puede perderse si se emite antes de empezar a escuchar).
	const [primero, setPrimero] = useState<EstadoReceptor | null>(null);
	const evento = useReceptor();
	// «Recibir otro» aparta el último evento (el de «terminado»): si no, seguiría
	// mandando sobre lo que contesta `pasar_recibir_abrir`.
	const [apartado, setApartado] = useState<EstadoReceptor | null>(null);
	const estado = (evento !== apartado ? evento : null) ?? primero;
	// El deck abierto que se va a reemplazar, prestado a Rust hasta que acabe.
	const prestados = useRef<Devolver[]>([]);
	const devolver = useCallback(() => {
		for (const d of prestados.current.splice(0)) void d();
	}, []);
	const fase = estado?.fase;
	useEffect(() => {
		if (fase !== "oferta" && fase !== "recibiendo") devolver();
	}, [fase, devolver]);
	useEffect(() => devolver, [devolver]);

	const abrir = useCallback(() => {
		setFallo(null);
		setApartado(evento);
		setPrimero(null);
		call("pasar_recibir_abrir")
			.then(setPrimero)
			.catch((e) => {
				// Se ha cerrado antes de que terminara de abrirse: no es un fallo.
				if ((e as AppError | undefined)?.code !== "Cancelled")
					setFallo(textoDeError(e, t, mensaje));
			});
	}, [t, mensaje, evento]);

	// Al entrar, se abre; al salir, se cierra (y se corta lo que hubiera).
	useEffect(() => {
		abrir();
		return () => {
			void call("pasar_recibir_cerrar").catch(() => {});
		};
		// Solo al montar y desmontar.
	}, []);

	if (fallo) {
		return (
			<>
				<DialogCuerpo>
					<p className="p-1 text-destructive">{fallo}</p>
				</DialogCuerpo>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={onCerrar}>
						{t("Cerrar")}
					</Button>
					<Button size="sm" onClick={abrir}>
						{t("Volver a intentarlo")}
					</Button>
				</DialogFooter>
			</>
		);
	}
	if (!estado) {
		return (
			<DialogCuerpo>
				<div className="flex items-center justify-center gap-2 p-6 text-muted-foreground">
					<Icono icon="lucide:loader-2" className="size-4 animate-spin" />
					{t("Preparando…")}
				</div>
			</DialogCuerpo>
		);
	}
	switch (estado.fase) {
		case "esperando":
			return <Esperando estado={estado} onCerrar={onCerrar} />;
		case "caducado":
			return (
				<>
					<DialogCuerpo>
						<p className="p-1">{t("El código ha caducado.")}</p>
					</DialogCuerpo>
					<DialogFooter>
						<Button size="sm" variant="outline" onClick={onCerrar}>
							{t("Cerrar")}
						</Button>
						<Button size="sm" onClick={abrir}>
							{t("Código nuevo")}
						</Button>
					</DialogFooter>
				</>
			);
		case "peticion":
			return <Peticion estado={estado} />;
		case "conectado":
			return (
				<>
					<DialogCuerpo>
						<div className="flex flex-col items-center gap-2 p-6 text-center">
							<Icono icon="lucide:loader-2" className="size-5 animate-spin text-muted-foreground" />
							<p>
								{t("Conectado con «{nombre}». Esperando a que mande los decks…", {
									nombre: estado.remoto?.apodo || estado.remoto?.huella || "",
								})}
							</p>
						</div>
					</DialogCuerpo>
					<Pie onCancelar={() => void call("pasar_recibir_cancelar")} />
				</>
			);
		case "oferta":
			return <Oferta estado={estado} prestados={prestados} />;
		case "recibiendo":
			return (
				<>
					<DialogCuerpo>
						<Avance estado={estado} />
						<Resultados resultados={estado.resultados} />
					</DialogCuerpo>
					<Pie onCancelar={() => void call("pasar_recibir_cancelar")} />
				</>
			);
		default:
			return <Final estado={estado} onCerrar={onCerrar} onOtro={abrir} />;
	}
}

function Pie({ onCancelar }: { onCancelar: () => void }) {
	const { t } = useLang();
	return (
		<DialogFooter>
			<Button size="sm" variant="outline" onClick={onCancelar}>
				{t("Cancelar")}
			</Button>
		</DialogFooter>
	);
}

/** Lo más largo que puede ser (`MAX_APODO` en Rust). */
const MAX_NOMBRE = 32;

/**
 * Con qué nombre te ven los demás equipos. Se guarda al salir del campo o con
 * Intro, y también si se cierra con algo escrito. Vacío vuelve al del sistema.
 */
function NombreEquipo() {
	const { t, mensaje } = useLang();
	const [nombre, setNombre] = useState("");
	const actual = useRef("");
	const guardado = useRef<string | null>(null);
	const guardar = useCallback(async () => {
		if (guardado.current === null || actual.current.trim() === guardado.current) return;
		const pedido = actual.current;
		guardado.current = pedido.trim();
		try {
			const queda = await call("pasar_nombre_set", { nombre: pedido });
			guardado.current = queda;
			// Si sigue escrito lo mismo, se enseña lo que ha quedado.
			if (actual.current === pedido) {
				actual.current = queda;
				setNombre(queda);
			}
		} catch (e) {
			toast.error(mensaje(e));
		}
	}, [mensaje]);
	useEffect(() => {
		let vivo = true;
		call("pasar_nombre", {})
			.then((n) => {
				if (!vivo) return;
				guardado.current = n;
				actual.current = n;
				setNombre(n);
			})
			.catch(() => {});
		return () => {
			vivo = false;
			void guardar();
		};
	}, [guardar]);
	return (
		<label className="flex w-full flex-col gap-1">
			<span className="text-muted-foreground">{t("Nombre de este equipo")}</span>
			<Input
				value={nombre}
				maxLength={MAX_NOMBRE}
				onChange={(e) => {
					actual.current = e.target.value;
					setNombre(e.target.value);
				}}
				onBlur={() => void guardar()}
				onKeyDown={(e) => {
					if (e.key === "Enter") void guardar();
				}}
				className="text-xs select-text"
			/>
		</label>
	);
}

/** Nada ha llegado aún: el nombre, el QR, el código, el equipo en la red local. */
function Esperando({ estado, onCerrar }: { estado: EstadoReceptor; onCerrar: () => void }) {
	const { t, mensaje } = useLang();
	const [ahora, setAhora] = useState(() => Date.now());
	const puedeCompartir = useEsMovil();
	useEffect(() => {
		const id = window.setInterval(() => setAhora(Date.now()), 1000);
		return () => window.clearInterval(id);
	}, []);
	const compartir = () => {
		call("pasar_compartir_codigo", { texto: estado.codigoMensaje }).catch((e) =>
			toast.error(textoDeError(e, t, mensaje)),
		);
	};
	const copiar = () => {
		void navigator.clipboard
			.writeText(estado.codigoMensaje)
			.then(() => toast.success(t("Código copiado")))
			.catch(() => {});
	};
	return (
		<>
			<DialogCuerpo>
				<div className="flex flex-col items-center gap-3 p-1">
					<NombreEquipo />
					<p className="text-center text-muted-foreground">
						{t(
							"En el otro equipo, abre «Enviar» y lee este QR o pega el código. Si estáis lejos, mándale el código por mensaje: vale con datos móviles.",
						)}
					</p>
					<img
						src={urlSvg(estado.qr)}
						alt={t("Código QR para recibir un deck")}
						className="size-56 max-w-full rounded-xl bg-white p-1"
					/>
					<div className="flex w-full flex-col gap-1">
						<div className="flex items-center justify-between text-muted-foreground">
							<span>{t("O pega este código")}</span>
							<span className="tabular-nums">
								{t("Caduca en {tiempo}", { tiempo: cuentaAtras(estado.caduca, ahora) })}
							</span>
						</div>
						<div className="flex items-start gap-1.5">
							<code className="max-h-20 min-w-0 flex-1 overflow-hidden rounded-lg border bg-muted/40 p-2 text-[11px] break-all select-text">
								{estado.codigoMensaje}
							</code>
							<Button
								size="icon-sm"
								variant="outline"
								onClick={copiar}
								aria-label={t("Copiar el código")}
							>
								<Icono icon="lucide:copy" />
							</Button>
							{puedeCompartir && (
								<Button
									size="icon-sm"
									variant="outline"
									onClick={compartir}
									aria-label={t("Mandar el código por otra app")}
								>
									<Icono icon="lucide:share-2" />
								</Button>
							)}
						</div>
					</div>
					<div className="flex w-full flex-col gap-1 text-muted-foreground">
						<div className="flex items-center gap-1.5">
							<Icono
								icon={estado.distancia ? "lucide:globe" : "lucide:loader-2"}
								className={cn("size-3.5 shrink-0", !estado.distancia && "animate-spin")}
							/>
							{estado.distancia
								? t("Vale desde cualquier sitio con internet. Solo se puede usar una vez.")
								: t("Buscando internet: por ahora, el código solo vale en tu misma red.")}
						</div>
						<div className="flex items-center gap-1.5">
							<Icono
								icon={estado.cerca ? "lucide:wifi" : "lucide:wifi-off"}
								className="size-3.5 shrink-0"
							/>
							{estado.cerca
								? t("Visible para los equipos de tu red local.")
								: t("No visible en la red local: hace falta el QR o el código.")}
						</div>
						<p>{t("Deja esta pantalla abierta hasta que el otro equipo se conecte.")}</p>
					</div>
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

/** Alguien de la red local quiere enviar: el mismo código en los dos equipos. */
function Peticion({ estado }: { estado: EstadoReceptor }) {
	const { t } = useLang();
	return (
		<>
			<DialogCuerpo>
				<div className="flex flex-col items-center gap-3 p-4 text-center">
					<Icono icon="lucide:monitor-smartphone" className="size-8 text-muted-foreground" />
					<p>
						{t("«{nombre}» quiere enviarte decks.", {
							nombre: estado.remoto?.apodo || estado.remoto?.huella || "",
						})}
					</p>
					<div className="text-3xl font-semibold tracking-widest tabular-nums select-text">
						{formatoCifras(estado.cifras ?? "")}
					</div>
					<p className="text-muted-foreground">
						{t("Acéptalo solo si el otro equipo enseña este mismo código.")}
					</p>
				</div>
			</DialogCuerpo>
			<DialogFooter>
				<Button
					size="sm"
					variant="outline"
					onClick={() => void call("pasar_peticion", { aceptar: false })}
				>
					{t("Rechazar")}
				</Button>
				<Button size="sm" onClick={() => void call("pasar_peticion", { aceptar: true })}>
					{t("Aceptar")}
				</Button>
			</DialogFooter>
		</>
	);
}

/** La oferta: elegir qué se quiere de cada deck. */
function Oferta({
	estado,
	prestados,
}: {
	estado: EstadoReceptor;
	prestados: RefObject<Devolver[]>;
}) {
	const { t, mensaje } = useLang();
	const [elegidos, setElegidos] = useState<Elegido[]>(() => eleccionesIniciales(estado.oferta));
	const [enviando, setEnviando] = useState(false);
	const cambiar = (i: number, e: Elegido) => setElegidos((v) => v.map((x, j) => (j === i ? e : x)));
	const listo = puedeResponder(estado.oferta, elegidos);
	const responder = async () => {
		setEnviando(true);
		// Si se reemplaza el deck abierto, lo pendiente se guarda antes y la
		// pantalla no lo vuelve a guardar hasta que Rust termine.
		for (const id of reemplazados(estado.oferta, elegidos)) {
			const d = await prestarDeck(id, { conservar: false }).catch(() => null);
			if (d) prestados.current.push(d);
		}
		call("pasar_responder", { decisiones: aDecisiones(estado.oferta, elegidos) }).catch((e) => {
			for (const d of prestados.current.splice(0)) void d();
			toast.error(textoDeError(e, t, mensaje));
			setEnviando(false);
		});
	};
	return (
		<>
			<DialogCuerpo>
				<p className="p-1 text-muted-foreground">
					{t("«{nombre}» te manda estos decks. Elige qué quieres de cada uno.", {
						nombre: estado.remoto?.apodo || estado.remoto?.huella || "",
					})}
				</p>
				<OfertaDecks oferta={estado.oferta} elegidos={elegidos} onCambio={cambiar} />
				<p className="p-1 text-muted-foreground">
					{t("Si estás con datos móviles, lo que recibas gasta de tu tarifa.")}
				</p>
			</DialogCuerpo>
			<DialogFooter>
				<Button size="sm" variant="outline" onClick={() => void call("pasar_recibir_cancelar")}>
					{t("Cancelar")}
				</Button>
				<Button size="sm" onClick={() => void responder()} disabled={!listo || enviando}>
					{quedaAlgo(elegidos) ? t("Recibir") : t("No recibir nada")}
				</Button>
			</DialogFooter>
		</>
	);
}

function Avance({ estado }: { estado: EstadoReceptor }) {
	const { t, lang } = useLang();
	const { tactil } = useFormato();
	const p = estado.progreso;
	return (
		<div className="flex flex-col gap-1.5 p-1">
			<div className="flex items-center justify-between">
				<span className="truncate">
					{p
						? t("Recibiendo «{nombre}» ({i}/{total})", {
								nombre: p.nombre,
								i: Math.min(p.decksHechos + 1, p.decksTotal),
								total: p.decksTotal,
							})
						: t("Recibiendo…")}
				</span>
				<span className="tabular-nums text-muted-foreground">{porcentaje(p)} %</span>
			</div>
			<Progress value={porcentaje(p)} />
			{p && (
				<span className="tabular-nums text-muted-foreground">
					{pesar(p.hechos, lang)} / {pesar(p.total, lang)}
				</span>
			)}
			{tactil && (
				<span className="text-muted-foreground">{t("Deja la app abierta hasta que termine.")}</span>
			)}
		</div>
	);
}

/** Terminó (bien, cancelado o con un fallo): el resultado y, si falta, las imágenes. */
function Final({
	estado,
	onCerrar,
	onOtro,
}: {
	estado: EstadoReceptor;
	onCerrar: () => void;
	onOtro: () => void;
}) {
	const { t, mensaje } = useLang();
	const { avance, fallidas, pedir } = useBajarImagenes();
	const pedidas = useRef(false);
	// Los decks que llegaron «solo originales» piden sus imágenes una vez.
	useEffect(() => {
		if (pedidas.current || estado.fase !== "terminado") return;
		pedidas.current = true;
		const ids = estado.resultados
			.filter((r) => r.estado === "hecho" && r.deckId && r.faltanImagenes > 0)
			.map((r) => r.deckId as string);
		if (ids.length > 0) pedir(ids);
		void import("@/features/library/store").then(({ useBiblioteca }) =>
			useBiblioteca.getState().refrescar(),
		);
	}, [estado, pedir]);
	const abrir = async (r: ResultadoPasar) => {
		if (!r.deckId) return;
		try {
			await abrirDeck(r.deckId);
			useUi.getState().setVista("deck");
			onCerrar();
		} catch (e) {
			toast.error(mensaje(e));
		}
	};
	const titulo =
		estado.fase === "terminado"
			? t("Listo.")
			: estado.fase === "cancelado"
				? t("Se ha cancelado. Lo que ya había llegado entero se ha guardado.")
				: "";
	return (
		<>
			<DialogCuerpo>
				<div className="flex flex-col gap-2 p-1">
					{titulo && <p className="font-medium">{titulo}</p>}
					{estado.error && (
						<p className="text-destructive">{textoDeError(estado.error, t, mensaje)}</p>
					)}
					<Resultados resultados={estado.resultados} onAbrir={abrir} />
					<ProgresoImagenes avance={avance} fallidas={fallidas} />
				</div>
			</DialogCuerpo>
			<DialogFooter>
				<Button size="sm" variant="outline" onClick={onCerrar}>
					{t("Cerrar")}
				</Button>
				<Button size="sm" onClick={onOtro}>
					{t("Recibir otro")}
				</Button>
			</DialogFooter>
		</>
	);
}
