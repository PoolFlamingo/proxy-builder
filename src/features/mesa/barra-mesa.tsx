import {
	Dices,
	Expand,
	Flag,
	History,
	Image,
	Keyboard,
	LogOut,
	Pause,
	Pencil,
	Play,
	Redo2,
	RotateCcw,
	SquareStack,
	Undo2,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { hacer } from "./acciones";
import * as op from "./operaciones";
import { usePantalla } from "./pantalla";
import { BotonSparring } from "./sparring/boton";

/** Minutos y segundos desde `desde` (ms). */
export function cronometro(desde: number, ahora: number): string {
	const s = Math.max(0, Math.floor((ahora - desde) / 1000));
	const m = Math.floor(s / 60);
	return `${m}:${String(s % 60).padStart(2, "0")}`;
}

function Boton({
	ayuda,
	onClick,
	children,
	activo,
	deshabilitado,
}: {
	ayuda: string;
	onClick: () => void;
	children: React.ReactNode;
	activo?: boolean;
	deshabilitado?: boolean;
}) {
	return (
		<WithTooltip content={ayuda}>
			<Button
				size="icon-sm"
				variant={activo ? "secondary" : "ghost"}
				onClick={onClick}
				aria-label={ayuda}
				disabled={deshabilitado}
				className="text-white hover:bg-white/10 hover:text-white"
			>
				{children}
			</Button>
		</WithTooltip>
	);
}

/**
 * **La barra de la mesa** (`capturas/pablo/00.jpg`): turno y ronda con su
 * reloj, pausa, deshacer y rehacer, azar, fichas, registro, tapete, atajos,
 * el sparring (en solitario) y lo de la partida (reiniciar, terminar, editar
 * el deck, salir).
 */
export function BarraMesa({
	onSalir,
	onEditarDeck,
	onReiniciar,
}: {
	onSalir: () => void;
	onEditarDeck: () => void;
	onReiniciar: () => void;
}) {
	const { t } = useLang();
	const turno = useMesa((s) => s.general?.turno ?? null);
	const pausada = useMesa((s) => s.general?.pausada ?? false);
	const mulligan = useMesa((s) => s.general?.mulligan != null);
	// El turno cuenta desde la hora del anfitrión: aquí se corrige con el
	// desfase que mide Rust (0 en solitario y en el propio anfitrión).
	const desfase = useRed((s) => s.desfaseMs);
	const [ahora, setAhora] = useState(Date.now() + desfase);
	useEffect(() => {
		setAhora(Date.now() + desfase);
		if (pausada) return;
		const i = window.setInterval(() => setAhora(Date.now() + desfase), 1000);
		return () => window.clearInterval(i);
	}, [pausada, desfase]);
	const abrir = usePantalla((s) => s.abrir);
	// En red: el sparring no va, reiniciar y terminar son del anfitrión, y
	// quien mira no toca nada de la partida.
	const papel = useRed((s) => s.papel);
	const enRed = papel !== null;
	const manda = papel === null || papel === "anfitrion";
	const juega = papel !== "espectador";

	return (
		<div
			className="absolute top-2 left-1/2 z-[7000] flex -translate-x-1/2 items-center gap-1 rounded-xl border border-white/10 bg-black/60 px-2 py-1 text-white shadow-lg backdrop-blur-md"
			data-tutorial="mesa-barra"
		>
			<span className="px-1.5 text-xs font-medium whitespace-nowrap tabular-nums">
				{mulligan || !turno || turno.numero === 0
					? t("Mulligan")
					: t("Turno {n} · ronda {r}", { n: turno.numero, r: turno.ronda })}
			</span>
			{turno && turno.numero > 0 && (
				<span className="text-[11px] text-white/60 tabular-nums">
					{cronometro(Number(turno.desdeMs), ahora)}
				</span>
			)}
			{juega && (
				<>
					<Boton
						ayuda={pausada ? t("Seguir") : t("Pausar")}
						onClick={() => void hacer({ que: "pausar", si: !pausada })}
						activo={pausada}
					>
						{pausada ? <Play className="size-4" /> : <Pause className="size-4" />}
					</Boton>
					<span className="mx-0.5 h-5 w-px bg-white/15" />
					<Boton ayuda={t("Deshacer (Ctrl+Z)")} onClick={() => void op.deshacer()}>
						<Undo2 className="size-4" />
					</Boton>
					<Boton ayuda={t("Rehacer (Ctrl+Y)")} onClick={() => void op.rehacer()}>
						<Redo2 className="size-4" />
					</Boton>
					<span className="mx-0.5 h-5 w-px bg-white/15" />
					<Boton ayuda={t("Dados y monedas")} onClick={() => abrir({ tipo: "azar" })}>
						<Dices className="size-4" />
					</Boton>
					<Boton ayuda={t("Crear ficha")} onClick={() => abrir({ tipo: "fichas" })}>
						<SquareStack className="size-4" />
					</Boton>
				</>
			)}
			<Boton ayuda={t("Registro de la partida")} onClick={() => abrir({ tipo: "registro" })}>
				<History className="size-4" />
			</Boton>
			{juega && (
				<Boton ayuda={t("Tapete")} onClick={() => abrir({ tipo: "tapete" })}>
					<Image className="size-4" />
				</Boton>
			)}
			<Boton ayuda={t("Atajos de teclado (?)")} onClick={() => abrir({ tipo: "atajos" })}>
				<Keyboard className="size-4" />
			</Boton>
			<Boton ayuda={t("Pantalla completa")} onClick={() => void pantallaCompleta()}>
				<Expand className="size-4" />
			</Boton>
			{!enRed && <BotonSparring />}
			<span className="mx-0.5 h-5 w-px bg-white/15" />
			{manda && (
				<>
					<Boton ayuda={t("Reiniciar (misma lista, nueva partida)")} onClick={onReiniciar}>
						<RotateCcw className="size-4" />
					</Boton>
					<Boton ayuda={t("Terminar la partida")} onClick={() => abrir({ tipo: "terminar" })}>
						<Flag className="size-4" />
					</Boton>
				</>
			)}
			{!enRed && (
				<Boton ayuda={t("Editar este deck")} onClick={onEditarDeck}>
					<Pencil className="size-4" />
				</Boton>
			)}
			<Boton ayuda={t("Salir de la mesa")} onClick={onSalir}>
				<LogOut className="size-4" />
			</Boton>
		</div>
	);
}

async function pantallaCompleta() {
	try {
		const { getCurrentWindow } = await import("@tauri-apps/api/window");
		const v = getCurrentWindow();
		await v.setFullscreen(!(await v.isFullscreen()));
	} catch {
		/* sin permiso o fuera de Tauri */
	}
}
