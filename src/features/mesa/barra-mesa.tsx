import { useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { AyudaAlTocar } from "@/components/ayuda-al-tocar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { call } from "@/lib/ipc";
import { esMovil } from "@/lib/plataforma";
import { cn } from "@/lib/utils";
import { useChat } from "@/stores/chat-store";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { hacer } from "./acciones";
import { useMesaMovil } from "./formato-mesa";
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
	className,
}: {
	ayuda: string;
	onClick: () => void;
	children: React.ReactNode;
	activo?: boolean;
	deshabilitado?: boolean;
	/** Un número en una esquina (los mensajes sin leer). */
	insignia?: number;
	className?: string;
}) {
	return (
		<WithTooltip content={ayuda}>
			<Button
				size="icon-sm"
				variant={activo ? "secondary" : "ghost"}
				onClick={onClick}
				aria-label={ayuda}
				disabled={deshabilitado}
				className={cn("text-white hover:bg-white/10 hover:text-white", className)}
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
export function BarraMesa(props: PropsBarra) {
	const movil = useMesaMovil();
	return movil ? <BarraMovil {...props} /> : <BarraEscritorio {...props} />;
}

type PropsBarra = {
	onSalir: () => void;
	onEditarDeck: () => void;
	onReiniciar: () => void;
};

function BarraEscritorio({ onSalir, onEditarDeck, onReiniciar }: PropsBarra) {
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
	const diaNoche = useMesa((s) => s.general?.designaciones.diaNoche ?? null);
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
			className="absolute top-2 left-1/2 z-[7000] flex w-max max-w-[calc(100%-1rem)] -translate-x-1/2 flex-wrap items-center justify-center gap-1 rounded-xl border border-white/10 bg-black/60 px-2 py-1 text-white shadow-lg backdrop-blur-md"
			data-tutorial="mesa-barra"
		>
			{!juega && (
				<span className="flex items-center gap-1 rounded-md bg-primary/20 px-1.5 py-0.5 text-[11px] font-medium text-primary">
					<Icono icon="lucide:eye" className="size-3.5" />
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
			{diaNoche && (
				<WithTooltip content={diaNoche === "dia" ? t("Es de día") : t("Es de noche")}>
					<span
						className={cn(
							"flex items-center rounded-md px-1 py-0.5",
							diaNoche === "dia" ? "text-amber-300" : "text-indigo-300",
						)}
					>
						<Icono
							icon={diaNoche === "dia" ? "game-icons:sun" : "game-icons:moon"}
							className="size-4"
						/>
					</span>
				</WithTooltip>
			)}
			{juega && (
				<>
					<Boton
						ayuda={pausada ? t("Seguir") : t("Pausar")}
						onClick={() => void hacer({ que: "pausar", si: !pausada })}
						activo={pausada}
					>
						{pausada ? (
							<Icono icon="lucide:play" className="size-4" />
						) : (
							<Icono icon="lucide:pause" className="size-4" />
						)}
					</Boton>
					<BotonResponder />
					<span className="mx-0.5 h-5 w-px bg-white/15" />
					<Boton ayuda={t("Deshacer (Ctrl+Z)")} onClick={() => void op.deshacer()}>
						<Icono icon="lucide:undo-2" className="size-4" />
					</Boton>
					<Boton ayuda={t("Rehacer (Ctrl+Y)")} onClick={() => void op.rehacer()}>
						<Icono icon="lucide:redo-2" className="size-4" />
					</Boton>
					<span className="mx-0.5 h-5 w-px bg-white/15" />
					<Boton ayuda={t("Dados y monedas")} onClick={() => abrir({ tipo: "azar" })}>
						<Icono icon="game-icons:rolling-dices" className="size-4" />
					</Boton>
					{commander && (
						<Boton
							ayuda={t("Daño de comandante")}
							onClick={() => abrir({ tipo: "danoComandante" })}
						>
							<Icono icon="game-icons:crown" className="size-4" />
						</Boton>
					)}
					<Boton ayuda={t("Crear ficha")} onClick={() => abrir({ tipo: "fichas" })}>
						<Icono icon="lucide:square-stack" className="size-4" />
					</Boton>
				</>
			)}
			{espectadores.length > 0 && (
				<AyudaAlTocar ayuda={t("Mirando: {nombres}", { nombres: espectadores.join(", ") })}>
					<span
						className="flex items-center gap-1 px-1 text-[11px] text-white/70 tabular-nums"
						role="img"
						aria-label={t("Mirando: {nombres}", { nombres: espectadores.join(", ") })}
					>
						<Icono icon="lucide:eye" className="size-3.5" />
						{espectadores.length}
					</span>
				</AyudaAlTocar>
			)}
			<Boton
				ayuda={t("Cementerios y exilios")}
				onClick={() => abrir({ tipo: "zona", jugador: viendo, zona: "cementerio" })}
			>
				<Icono icon="game-icons:tombstone" className="size-4" />
			</Boton>
			<Boton
				ayuda={t("Registro de la partida")}
				activo={panel === "registro"}
				onClick={() => setPanel(panel === "registro" ? null : "registro")}
			>
				<Icono icon="lucide:history" className="size-4" />
			</Boton>
			<Boton
				ayuda={t("Notas")}
				activo={panel === "notas"}
				insignia={panel === "notas" ? 0 : nNotas}
				onClick={() => setPanel(panel === "notas" ? null : "notas")}
			>
				<Icono icon="lucide:sticky-note" className="size-4" />
			</Boton>
			{(enRed || varios) && (
				<Boton
					ayuda={t("Chat")}
					activo={panel === "chat"}
					insignia={panel === "chat" ? 0 : sinLeer}
					onClick={() => setPanel(panel === "chat" ? null : "chat")}
				>
					<Icono icon="lucide:message-square" className="size-4" />
				</Boton>
			)}
			{juega && (
				<Boton ayuda={t("Tapete")} onClick={() => abrir({ tipo: "tapete" })}>
					<Icono icon="lucide:image" className="size-4" />
				</Boton>
			)}
			{/* Con el dedo no hay teclado: el botón de los atajos no se enseña. */}
			<Boton
				ayuda={t("Atajos de teclado (?)")}
				onClick={() => abrir({ tipo: "atajos" })}
				className="[[data-tactil]_&]:hidden"
			>
				<Icono icon="lucide:keyboard" className="size-4" />
			</Boton>
			<Boton ayuda={t("Pantalla completa")} onClick={() => void pantallaCompleta()}>
				<Icono icon="lucide:expand" className="size-4" />
			</Boton>
			<SelectorDisposicion className="text-white hover:bg-white/10 hover:text-white" />
			{!enRed && <BotonSparring />}
			<span className="mx-0.5 h-5 w-px bg-white/15" />
			{manda && (
				<>
					<Boton ayuda={t("Reiniciar (misma lista, nueva partida)")} onClick={onReiniciar}>
						<Icono icon="lucide:rotate-ccw" className="size-4" />
					</Boton>
					<Boton ayuda={t("Terminar la partida")} onClick={() => abrir({ tipo: "terminar" })}>
						<Icono icon="game-icons:checkered-flag" className="size-4" />
					</Boton>
				</>
			)}
			{!enRed && (
				<Boton ayuda={t("Editar este deck")} onClick={onEditarDeck}>
					<Icono icon="lucide:pencil" className="size-4" />
				</Boton>
			)}
			<Boton ayuda={t("Salir de la mesa")} onClick={onSalir}>
				<Icono icon="lucide:log-out" className="size-4" />
			</Boton>
		</div>
	);
}

/** Si las barras del sistema de Android están escondidas (la pantalla completa de la mesa). */
let completaAndroid = false;

async function pantallaCompleta() {
	try {
		// En el móvil la ventana ya ocupa todo: lo que se esconde son las barras del sistema.
		if (await esMovil()) {
			const si = !completaAndroid;
			await call("app_pantalla_completa", { si });
			// Solo si se pudo: si falla, sigue como estaba.
			completaAndroid = si;
			return;
		}
		const { getCurrentWindow } = await import("@tauri-apps/api/window");
		const v = getCurrentWindow();
		await v.setFullscreen(!(await v.isFullscreen()));
	} catch {
		/* sin permiso o fuera de Tauri */
	}
}

/** Lo que enseña la barra del móvil: el turno y los botones de lo que más se usa; el resto, en «⋯». */
function BarraMovil({ onSalir, onEditarDeck, onReiniciar }: PropsBarra) {
	const { t } = useLang();
	const turno = useMesa((s) => s.general?.turno ?? null);
	const pausada = useMesa((s) => s.general?.pausada ?? false);
	const mulligan = useMesa((s) => s.general?.mulligan != null);
	const commander = useMesa((s) => s.formato === "commander");
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
	const diaNoche = useMesa((s) => s.general?.designaciones.diaNoche ?? null);
	const papel = useRed((s) => s.papel);
	const enRed = papel !== null;
	const manda = papel === null || papel === "anfitrion";
	const juega = papel !== "espectador";
	const espectadores = useRed(
		useShallow((s) => s.sala.filter((x) => x.rol === "espectador").map((x) => x.apodo)),
	);
	return (
		<div
			className="absolute top-2 left-1/2 z-[7000] flex w-max max-w-[calc(100%-8rem)] -translate-x-1/2 flex-wrap items-center justify-center gap-0.5 rounded-xl border border-white/10 bg-black/60 px-1 py-0.5 text-white shadow-lg backdrop-blur-md"
			data-tutorial="mesa-barra"
		>
			<Boton ayuda={t("Salir de la mesa")} onClick={onSalir}>
				<Icono icon="lucide:log-out" className="size-5" />
			</Boton>
			<span className="px-1.5 text-xs font-medium whitespace-nowrap tabular-nums">
				{mulligan || !turno || turno.numero === 0
					? t("Mulligan")
					: t("Turno {n} · ronda {r}", { n: turno.numero, r: turno.ronda })}
				{turno && turno.numero > 0 && (
					<span className="ml-1.5 text-[11px] text-white/60">
						{cronometro(Number(turno.desdeMs), ahora)}
					</span>
				)}
			</span>
			{diaNoche && (
				<span
					className={cn("px-1", diaNoche === "dia" ? "text-amber-300" : "text-indigo-300")}
					role="img"
					aria-label={diaNoche === "dia" ? t("Es de día") : t("Es de noche")}
				>
					<Icono
						icon={diaNoche === "dia" ? "game-icons:sun" : "game-icons:moon"}
						className="size-4"
					/>
				</span>
			)}
			{espectadores.length > 0 && (
				<AyudaAlTocar ayuda={t("Mirando: {nombres}", { nombres: espectadores.join(", ") })}>
					<span
						className="flex items-center gap-1 px-1 text-[11px] text-white/70 tabular-nums"
						role="img"
						aria-label={t("Mirando: {nombres}", { nombres: espectadores.join(", ") })}
					>
						<Icono icon="lucide:eye" className="size-3.5" />
						{espectadores.length}
					</span>
				</AyudaAlTocar>
			)}
			{juega && (
				<>
					<Boton ayuda={t("Deshacer")} onClick={() => void op.deshacer()}>
						<Icono icon="lucide:undo-2" className="size-5" />
					</Boton>
					<Boton ayuda={t("Rehacer")} onClick={() => void op.rehacer()}>
						<Icono icon="lucide:redo-2" className="size-5" />
					</Boton>
					<BotonResponder />
					<Boton ayuda={t("Mano")} onClick={() => usePantalla.getState().setCapa("mano")}>
						<Icono icon="lucide:hand" className="size-5" />
					</Boton>
				</>
			)}
			<Boton ayuda={t("Vida")} onClick={() => usePantalla.getState().setCapa("vida")}>
				<Icono icon="game-icons:hearts" className="size-5" />
			</Boton>
			<Boton
				ayuda={t("Registro de la partida")}
				onClick={() => setPanel(panel === "registro" ? null : "registro")}
			>
				<Icono icon="lucide:history" className="size-5" />
			</Boton>
			{(enRed || varios) && (
				<Boton
					ayuda={t("Chat")}
					insignia={panel === "chat" ? 0 : sinLeer}
					onClick={() => setPanel(panel === "chat" ? null : "chat")}
				>
					<Icono icon="lucide:message-square" className="size-5" />
				</Boton>
			)}
			{!enRed && <BotonSparring />}
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button
						size="icon-sm"
						variant="ghost"
						aria-label={t("Más")}
						className="text-white hover:bg-white/10 hover:text-white"
					>
						<Icono icon="lucide:ellipsis" className="size-5" />
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="end" className="z-[9000] w-64 text-xs">
					{juega && (
						<>
							<DropdownMenuItem onSelect={() => void hacer({ que: "pausar", si: !pausada })}>
								<Icono icon={pausada ? "lucide:play" : "lucide:pause"} />
								{pausada ? t("Seguir") : t("Pausar")}
							</DropdownMenuItem>
							<DropdownMenuItem onSelect={() => abrir({ tipo: "azar" })}>
								<Icono icon="game-icons:rolling-dices" />
								{t("Dados y monedas")}
							</DropdownMenuItem>
							{commander && (
								<DropdownMenuItem onSelect={() => abrir({ tipo: "danoComandante" })}>
									<Icono icon="game-icons:crown" />
									{t("Daño de comandante")}
								</DropdownMenuItem>
							)}
							<DropdownMenuItem onSelect={() => abrir({ tipo: "fichas" })}>
								<Icono icon="lucide:square-stack" />
								{t("Crear ficha")}
							</DropdownMenuItem>
						</>
					)}
					<DropdownMenuItem
						onSelect={() => abrir({ tipo: "zona", jugador: viendo, zona: "cementerio" })}
					>
						<Icono icon="game-icons:tombstone" />
						{t("Cementerios y exilios")}
					</DropdownMenuItem>
					<DropdownMenuItem onSelect={() => setPanel("notas")}>
						<Icono icon="lucide:sticky-note" />
						{t("Notas")}
						{nNotas > 0 && <span className="ml-auto tabular-nums">{nNotas}</span>}
					</DropdownMenuItem>
					{juega && (
						<DropdownMenuItem onSelect={() => abrir({ tipo: "tapete" })}>
							<Icono icon="lucide:image" />
							{t("Tapete")}
						</DropdownMenuItem>
					)}
					<DropdownMenuItem onSelect={() => void pantallaCompleta()}>
						<Icono icon="lucide:expand" />
						{t("Pantalla completa")}
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					{manda && (
						<>
							<DropdownMenuItem onSelect={onReiniciar}>
								<Icono icon="lucide:rotate-ccw" />
								{t("Reiniciar (misma lista, nueva partida)")}
							</DropdownMenuItem>
							<DropdownMenuItem onSelect={() => abrir({ tipo: "terminar" })}>
								<Icono icon="game-icons:checkered-flag" />
								{t("Terminar la partida")}
							</DropdownMenuItem>
						</>
					)}
					{!enRed && (
						<DropdownMenuItem onSelect={onEditarDeck}>
							<Icono icon="lucide:pencil" />
							{t("Editar este deck")}
						</DropdownMenuItem>
					)}
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}
