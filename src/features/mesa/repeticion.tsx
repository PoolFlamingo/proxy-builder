import {
	ChevronsLeft,
	ChevronsRight,
	LogOut,
	Pause,
	Play,
	SkipBack,
	SkipForward,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Selector } from "@/components/campos";
import { traduccionSuelta, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { mensajeDeError } from "@/lib/errores";
import { call } from "@/lib/ipc";
import { useMesa } from "@/stores/mesa-store";
import { useRepeticion } from "@/stores/repeticion-store";
import { useUi } from "@/stores/ui-store";
import { suscribir, suscrito } from "./conexion";
import { formatoDuracion } from "./cuentas-fin";
import {
	duracionMovimiento,
	hitoAnterior,
	hitoSiguiente,
	pausaEntre,
	VELOCIDADES,
} from "./cuentas-repeticion";
import { olvidarSitios } from "./flip";

/**
 * **La repetición de una partida terminada** (H3): la mesa en «solo mirar»
 * con una línea de tiempo. Rust (`mesa/repeticion.rs`) calcula la partida de
 * cada punto; aquí solo se elige a dónde ir, a qué ritmo y como quién se mira.
 */
const TODO = "todo";

/** Abre una partida del historial y va a la mesa a verla. */
export async function verRepeticionDelHistorial(id: string): Promise<boolean> {
	try {
		const v = await call("repeticion_abrir", { id });
		useRepeticion.getState().poner(v);
		if (!suscrito()) await suscribir();
		useUi.getState().setVista("mesa");
		return true;
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
		return false;
	}
}

/** Repite la partida que acaba de terminar, sin salir de la mesa. */
export async function verRepeticionActual(): Promise<boolean> {
	try {
		useRepeticion.getState().poner(await call("repeticion_actual"));
		return true;
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
		return false;
	}
}

export async function salirDeLaRepeticion() {
	try {
		await call("repeticion_salir");
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
	}
	const del_historial = useRepeticion.getState().activa?.desdeHistorial ?? false;
	useRepeticion.getState().poner(null);
	if (del_historial) useMesa.getState().cerrar();
}

/** La barra de abajo: tiempo, reproducir, saltos, velocidad y «ver como». */
export function BarraRepeticion() {
	const { t } = useLang();
	const rep = useRepeticion((s) => s.activa);
	const poner = useRepeticion((s) => s.poner);
	const jugadores = useMesa((s) => s.jugadores);
	const asientos = useMesa((s) => s.asientos);
	const [reproduciendo, setReproduciendo] = useState(false);
	const [velocidad, setVelocidad] = useState(1);
	// Se pide solo el último destino mientras se arrastra: no se encolan.
	const pedido = useRef<{ pos: number; como: number | null } | null>(null);
	const enCurso = useRef(false);
	const velocidadActual = useRef(velocidad);
	velocidadActual.current = velocidad;

	const ir = useCallback(
		async (pos: number, como: number | null) => {
			pedido.current = { pos, como };
			if (enCurso.current) return;
			enCurso.current = true;
			try {
				while (pedido.current) {
					const p = pedido.current;
					pedido.current = null;
					// Antes de pedirlo: la vista llega por el canal y las cartas ya tienen
					// que saber si se deslizan hasta su sitio nuevo y cuánto.
					const ahora = useRepeticion.getState();
					const ms =
						p.como === (ahora.activa?.como ?? null)
							? duracionMovimiento(ahora.activa?.pos ?? p.pos, p.pos, velocidadActual.current)
							: 0;
					// Un salto no se desliza: lo de antes no es de donde vienen las cartas.
					if (ms === 0) olvidarSitios();
					ahora.animar(ms);
					poner(await call("repeticion_ir", { pos: p.pos, como: p.como }));
				}
			} catch (e) {
				toast.error(mensajeDeError(e, traduccionSuelta));
				setReproduciendo(false);
			} finally {
				enCurso.current = false;
			}
		},
		[poner],
	);

	const pos = rep?.pos ?? 0;
	const total = rep?.total ?? 0;
	const como = rep?.como ?? null;

	// Reproducir: el siguiente evento llega tras el hueco real, acotado y acelerado.
	useEffect(() => {
		if (!reproduciendo || !rep) return;
		if (pos >= total) {
			setReproduciendo(false);
			return;
		}
		const espera = pausaEntre(rep.tMs[pos - 1], rep.tMs[pos], velocidad);
		const id = window.setTimeout(() => void ir(pos + 1, como), espera);
		return () => window.clearTimeout(id);
	}, [reproduciendo, rep, pos, total, velocidad, como, ir]);

	// Atajos: espacio, flechas, Inicio y Fin.
	useEffect(() => {
		const alTeclear = (e: KeyboardEvent) => {
			const t = e.target as HTMLElement | null;
			if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
			if (e.code === "Space") setReproduciendo((r) => !r);
			else if (e.code === "ArrowRight") void ir(Math.min(total, pos + 1), como);
			else if (e.code === "ArrowLeft") void ir(Math.max(0, pos - 1), como);
			else if (e.code === "Home") void ir(0, como);
			else if (e.code === "End") void ir(total, como);
			else return;
			e.preventDefault();
		};
		window.addEventListener("keydown", alTeclear);
		return () => window.removeEventListener("keydown", alTeclear);
	}, [ir, pos, total, como]);

	if (!rep) return null;
	const hito = [...rep.hitos].reverse().find((h) => h.pos <= pos);
	const desde = rep.tMs[0] ?? 0;
	const ahora = pos === 0 ? desde : (rep.tMs[pos - 1] ?? desde);
	const nombre = (id: number) => jugadores[id]?.nombre ?? "?";

	return (
		<div
			className="absolute bottom-2 left-1/2 z-[7000] flex w-[min(860px,94%)] -translate-x-1/2 flex-col gap-1.5 rounded-xl border border-white/10 bg-black/70 px-3 py-2 text-white shadow-lg backdrop-blur-md"
			data-tutorial="mesa-repeticion"
		>
			<div className="flex items-center gap-2 text-xs">
				<span className="font-medium">{t("Repetición")}</span>
				<span className="truncate text-white/70">
					{hito
						? t("Turno {n} · ronda {r}", { n: hito.turno, r: hito.ronda }) +
							" · " +
							nombre(hito.activo)
						: t("Antes del primer turno")}
				</span>
				<span className="ml-auto text-white/60 tabular-nums">
					{formatoDuracion(ahora - desde)} /{" "}
					{formatoDuracion((rep.tMs[total - 1] ?? desde) - desde)}
				</span>
			</div>
			<div className="relative">
				<Slider
					min={0}
					max={Math.max(1, total)}
					step={1}
					value={[pos]}
					onValueChange={([v]) => void ir(v ?? 0, como)}
					aria-label={t("Línea de tiempo")}
				/>
				{rep.hitos.map((h) => (
					<span
						key={h.pos}
						className="pointer-events-none absolute top-1/2 h-2.5 w-px -translate-y-1/2 bg-white/40"
						style={{ left: `${(h.pos / Math.max(1, total)) * 100}%` }}
					/>
				))}
			</div>
			<div className="flex items-center gap-1">
				<Boton ayuda={t("Al principio")} onClick={() => void ir(0, como)}>
					<SkipBack className="size-4" />
				</Boton>
				<Boton
					ayuda={t("Turno anterior")}
					onClick={() => void ir(hitoAnterior(rep.hitos, pos), como)}
				>
					<ChevronsLeft className="size-4" />
				</Boton>
				<Boton
					ayuda={reproduciendo ? t("Pausar") : t("Reproducir")}
					onClick={() => {
						if (!reproduciendo && pos >= total) void ir(0, como);
						setReproduciendo((r) => !r);
					}}
				>
					{reproduciendo ? <Pause className="size-4" /> : <Play className="size-4" />}
				</Boton>
				<Boton
					ayuda={t("Turno siguiente")}
					onClick={() => void ir(hitoSiguiente(rep.hitos, pos, total), como)}
				>
					<ChevronsRight className="size-4" />
				</Boton>
				<Boton ayuda={t("Al final")} onClick={() => void ir(total, como)}>
					<SkipForward className="size-4" />
				</Boton>
				<span className="mx-1 whitespace-nowrap text-[11px] text-white/60 tabular-nums">
					{pos} / {total}
				</span>
				<span className="ml-auto flex items-center gap-1.5 whitespace-nowrap text-[11px]">
					<span className="text-white/60">{t("Velocidad")}</span>
					<Selector<string>
						valor={String(velocidad)}
						onChange={(v) => setVelocidad(Number(v))}
						opciones={VELOCIDADES.map((v) => ({ valor: String(v), etiqueta: `${v}×` }))}
						ancho="w-20"
						ariaLabel={t("Velocidad")}
					/>
					<span className="text-white/60">{t("Ver como")}</span>
					<Selector<string>
						valor={como === null ? TODO : String(como)}
						onChange={(v) => void ir(pos, v === TODO ? null : Number(v))}
						opciones={[
							{ valor: TODO, etiqueta: t("Todo a la vista") },
							...asientos.map((id) => ({ valor: String(id), etiqueta: nombre(id) })),
						]}
						ancho="w-44"
						ariaLabel={t("Ver como")}
					/>
					<Button
						size="sm"
						variant="outline"
						className="h-7 border-white/20 bg-transparent text-white hover:bg-white/10 hover:text-white"
						onClick={() => void salirDeLaRepeticion()}
					>
						<LogOut className="size-4" />
						{t("Salir de la repetición")}
					</Button>
				</span>
			</div>
		</div>
	);
}

function Boton({
	ayuda,
	onClick,
	children,
}: {
	ayuda: string;
	onClick: () => void;
	children: React.ReactNode;
}) {
	return (
		<WithTooltip content={ayuda}>
			<Button
				size="icon-sm"
				variant="ghost"
				onClick={onClick}
				aria-label={ayuda}
				className="text-white hover:bg-white/10 hover:text-white"
			>
				{children}
			</Button>
		</WithTooltip>
	);
}
