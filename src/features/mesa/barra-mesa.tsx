import {
	Crown,
	Dices,
	Expand,
	Eye,
	Flag,
	History,
	Image,
	Keyboard,
	LogOut,
	MessageSquare,
	Pause,
	Pencil,
	Play,
	Redo2,
	RotateCcw,
	Skull,
	SquareStack,
	StickyNote,
	Undo2,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useChat } from "@/stores/chat-store";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { hacer } from "./acciones";
import * as op from "./operaciones";
import { usePantalla } from "./pantalla";
import { BotonResponder } from "./responder";
import { SelectorDisposicion } from "./selector-disposicion";
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
	insignia,
}: {
	ayuda: string;
	onClick: () => void;
	children: React.ReactNode;
	activo?: boolean;
	deshabilitado?: boolean;
	/** Un número en una esquina (los mensajes sin leer). */
	insignia?: number;
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
				{insignia ? (
					<span className="absolute -top-1 -right-1 min-w-4 rounded-full bg-primary px-1 text-[10px] leading-4 text-primary-foreground tabular-nums">
						{insignia}
					</span>
				) : null}
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
	const commander = useMesa((s) => s.formato === "commander");
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
	const viendo = useMesa(tableroDe);
	const panel = usePantalla((s) => s.panel);
	const setPanel = usePantalla((s) => s.setPanel);
	const sinLeer = useChat((s) => s.sinLeer);
	const nNotas = useMesa((s) => s.general?.notas.length ?? 0);
	const varios = useMesa((s) => s.asientos.length > 1);
	// En red: el sparring no va, reiniciar y terminar son del anfitrión, y
	// quien mira no toca nada de la partida.
	const papel = useRed((s) => s.papel);
	const enRed = papel !== null;
	const manda = papel === null || papel === "anfitrion";
	const juega = papel !== "espectador";
	const espectadores = useRed(
		useShallow((s) => s.sala.filter((x) => x.rol === "espectador").map((x) => x.apodo)),
	);

	return (
		<div
			className="absolute top-2 left-1/2 z-[7000] flex -translate-x-1/2 items-center gap-1 rounded-xl border border-white/10 bg-black/60 px-2 py-1 text-white shadow-lg backdrop-blur-md"
			data-tutorial="mesa-barra"
		>
			{!juega && (
				<span className="flex items-center gap-1 rounded-md bg-primary/20 px-1.5 py-0.5 text-[11px] font-medium text-primary">
					<Eye className="size-3.5" />
					{t("Mirando")}
				</span>
			)}
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
					<BotonResponder />
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
					{commander && (
						<Boton
							ayuda={t("Daño de comandante")}
							onClick={() => abrir({ tipo: "danoComandante" })}
						>
							<Crown className="size-4" />
						</Boton>
					)}
					<Boton ayuda={t("Crear ficha")} onClick={() => abrir({ tipo: "fichas" })}>
						<SquareStack className="size-4" />
					</Boton>
				</>
			)}
			{espectadores.length > 0 && (
				<WithTooltip content={t("Mirando: {nombres}", { nombres: espectadores.join(", ") })}>
					<span
						className="flex items-center gap-1 px-1 text-[11px] text-white/70 tabular-nums"
						role="img"
						aria-label={t("Mirando: {nombres}", { nombres: espectadores.join(", ") })}
					>
						<Eye className="size-3.5" />
						{espectadores.length}
					</span>
				</WithTooltip>
			)}
			<Boton
				ayuda={t("Cementerios y exilios")}
				onClick={() => abrir({ tipo: "zona", jugador: viendo, zona: "cementerio" })}
			>
				<Skull className="size-4" />
			</Boton>
			<Boton
				ayuda={t("Registro de la partida")}
				activo={panel === "registro"}
				onClick={() => setPanel(panel === "registro" ? null : "registro")}
			>
				<History className="size-4" />
			</Boton>
			<Boton
				ayuda={t("Notas")}
				activo={panel === "notas"}
				insignia={panel === "notas" ? 0 : nNotas}
				onClick={() => setPanel(panel === "notas" ? null : "notas")}
			>
				<StickyNote className="size-4" />
			</Boton>
			{(enRed || varios) && (
				<Boton
					ayuda={t("Chat")}
					activo={panel === "chat"}
					insignia={panel === "chat" ? 0 : sinLeer}
					onClick={() => setPanel(panel === "chat" ? null : "chat")}
				>
					<MessageSquare className="size-4" />
				</Boton>
			)}
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
			<SelectorDisposicion className="text-white hover:bg-white/10 hover:text-white" />
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
