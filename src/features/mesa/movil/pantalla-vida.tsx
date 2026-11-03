import { useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Dialog, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { hacer } from "../acciones";
import { comandantesDe } from "../dano-comandante";
import { MarcasJugador } from "../marcas-jugador";
import { usePantalla } from "../pantalla";
import {
	type Acumulador,
	accionDe,
	alternarObjetivo,
	type Cuenta,
	claveDe,
	crearAcumulador,
	crearMantener,
	type Mantener,
	mismoObjetivo,
	type Objetivo,
	objetivoQueCuenta,
	objetivosQueCuentan,
	PASO_LARGO,
	PASO_TOQUE,
	vidaDe,
} from "./cuentas-vida";

type Pestana = "vida" | "veneno" | "comandante";

/**
 * **La pantalla de vida del móvil** (plan de la mesa, B5.10 e inventario §22):
 * vida, veneno y daño de comandante a pantalla completa. − y + suman de uno en
 * uno con un toque y de diez en diez manteniendo; se pueden elegir varios
 * jugadores a la vez (y el fantasma del solitario) y el cambio va a todos.
 * Manda las mismas `Accion` que los marcadores del tablero
 * (`cuentas-vida.ts`); quién queda fuera lo sigue decidiendo el motor y lo
 * confirma la persona.
 */
export function PantallaVida() {
	const abierta = usePantalla((s) => s.capa === "vida");
	return (
		<Dialog open={abierta} onOpenChange={(o) => !o && usePantalla.getState().setCapa(null)}>
			{abierta && <ContenidoVida />}
		</Dialog>
	);
}

function ContenidoVida() {
	const { t } = useLang();
	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const fantasma = useMesa((s) => s.general?.fantasma ?? null);
	const verFantasma = useUi((s) => s.mesa.fantasma);
	const equipos = useMesa((s) => s.general?.equipos);
	const cartas = useMesa((s) => s.cartas);
	const viendo = useMesa(tableroDe);
	const comandantes = useMemo(() => comandantesDe(cartas), [cartas]);
	const hayFantasma = fantasma?.visible === true && verFantasma;
	const [elegidos, setElegidos] = useState<Objetivo[]>([{ tipo: "jugador", id: viendo }]);
	const [pestana, setPestana] = useState<Pestana>("vida");
	const [comandante, setComandante] = useState<number | null>(null);
	const [, repintar] = useState(0);
	const cuentaRef = useRef<Cuenta>({ que: "vida" });
	const objetivos = useRef(new Map<string, { o: Objetivo; c: Cuenta }>());
	const comandanteActual = comandante ?? comandantes[0]?.id ?? null;
	const cuenta: Cuenta =
		pestana === "comandante" && comandanteActual !== null
			? { que: "comandante", comandante: comandanteActual }
			: { que: pestana === "veneno" ? "veneno" : "vida" };
	cuentaRef.current = cuenta;

	// Lo que ya tiene un objetivo en una cuenta (para el daño de comandante, que no baja de 0).
	const valorDe = (o: Objetivo, c: Cuenta): number => {
		if (o.tipo === "fantasma") {
			if (!fantasma) return 0;
			if (c.que === "vida") return fantasma.vida;
			if (c.que === "veneno") return fantasma.veneno ?? 0;
			return fantasma.danoComandante[c.comandante] ?? 0;
		}
		const j = jugadores[o.id];
		if (!j) return 0;
		if (c.que === "vida") return vidaDe(j, equipos);
		if (c.que === "veneno") return j.veneno;
		return j.danoComandante[c.comandante] ?? 0;
	};
	const valorRef = useRef(valorDe);
	valorRef.current = valorDe;

	const acumulador = useRef<Acumulador | null>(null);
	if (acumulador.current === null) {
		acumulador.current = crearAcumulador(
			(clave, n) => {
				const x = objetivos.current.get(clave);
				if (!x) return;
				const accion = accionDe(x.o, x.c, n, valorRef.current(x.o, x.c));
				if (accion) void hacer(accion);
			},
			() => repintar((v) => v + 1),
		);
	}
	// Al cerrar, lo que se haya tocado y no se haya mandado se manda.
	useEffect(() => () => acumulador.current?.vaciar(), []);

	const aplicar = (delta: number) => {
		// Con vida compartida, un equipo es un solo objetivo aunque se elija a los dos.
		const c = cuentaRef.current;
		for (const o of objetivosQueCuentan(elegidos, c, jugadores, equipos)) {
			const clave = claveDe(o, c);
			objetivos.current.set(clave, { o, c });
			acumulador.current?.sumar(clave, delta);
		}
	};
	const pendienteDe = (o: Objetivo) =>
		acumulador.current?.pendiente(
			claveDe(objetivoQueCuenta(o, cuenta, jugadores, equipos), cuenta),
		) ?? 0;

	const solo = elegidos.length === 1 ? elegidos[0] : undefined;
	const nombreDe = (o: Objetivo) =>
		o.tipo === "fantasma" ? t("Fantasma") : (jugadores[o.id]?.nombre ?? "");
	const titulo =
		pestana === "vida" ? t("Vida") : pestana === "veneno" ? t("Veneno") : t("Daño de comandante");

	return (
		<DialogoAdaptable
			className="flex flex-col gap-2.5 sm:max-w-3xl"
			cambiosSinGuardar={false}
			data-tutorial="mesa-pantalla-vida"
		>
			<DialogTitle className="sr-only">{titulo}</DialogTitle>
			<DialogDescription className="sr-only">
				{t("Toca para sumar o restar de uno en uno; mantén pulsado para hacerlo de diez en diez.")}
			</DialogDescription>
			<div className="flex shrink-0 items-center gap-1.5 pr-10" role="tablist">
				{(
					[
						["vida", t("Vida")],
						["veneno", t("Veneno")],
						["comandante", t("Comandante")],
					] as const
				).map(([id, texto]) => (
					<button
						key={id}
						type="button"
						role="tab"
						aria-selected={pestana === id}
						disabled={id === "comandante" && comandantes.length === 0}
						onClick={() => setPestana(id)}
						className={cn(
							"min-h-11 flex-1 rounded-lg border px-2 text-sm font-medium disabled:opacity-40",
							pestana === id ? "border-primary bg-primary text-primary-foreground" : "bg-muted/40",
						)}
					>
						{texto}
					</button>
				))}
			</div>
			{pestana === "comandante" && comandantes.length > 0 && (
				<ScrollArea className="shrink-0" viewportClassName="max-w-full">
					<div className="flex gap-1.5 p-1">
						{comandantes.map((c) => (
							<button
								key={c.id}
								type="button"
								aria-pressed={comandanteActual === c.id}
								onClick={() => setComandante(c.id)}
								className={cn(
									"flex min-h-11 shrink-0 items-center gap-1.5 rounded-lg border px-2.5 text-xs",
									comandanteActual === c.id ? "border-primary bg-primary/15" : "bg-muted/40",
								)}
							>
								<Icono icon="game-icons:crown" className="size-4" />
								{c.nombre}
							</button>
						))}
					</div>
				</ScrollArea>
			)}
			<div className="flex min-h-0 flex-1 gap-2.5">
				<ScrollArea className="w-52 shrink-0" viewportClassName="h-full">
					<div className="flex flex-col gap-1 p-1">
						<p className="px-1 text-[11px] text-muted-foreground">
							{t("Elige a quién; puedes elegir a varios.")}
						</p>
						{asientos.map((id) => {
							const j = jugadores[id];
							if (!j) return null;
							const o: Objetivo = { tipo: "jugador", id };
							const sel = elegidos.some((x) => mismoObjetivo(x, o));
							const p = pendienteDe(o);
							return (
								<button
									key={id}
									type="button"
									aria-pressed={sel}
									onClick={() => setElegidos((e) => alternarObjetivo(e, o))}
									className={cn(
										"flex min-h-11 items-center gap-2 rounded-lg border px-2 py-1 text-left text-xs",
										sel ? "border-primary bg-primary/15" : "bg-muted/40",
										j.eliminado && "opacity-50",
									)}
								>
									<span className="size-3 shrink-0 rounded-full" style={{ background: j.color }} />
									<span className="flex min-w-0 flex-1 flex-col leading-tight">
										<span className="truncate font-medium">{j.nombre}</span>
										<MarcasJugador jugador={id} pequeno />
									</span>
									<Valor valor={valorDe(o, cuenta)} pendiente={p} />
								</button>
							);
						})}
						{hayFantasma && (
							<button
								type="button"
								aria-pressed={elegidos.some((x) => x.tipo === "fantasma")}
								onClick={() => setElegidos((e) => alternarObjetivo(e, { tipo: "fantasma" }))}
								className={cn(
									"flex min-h-11 items-center gap-2 rounded-lg border px-2 py-1 text-left text-xs",
									elegidos.some((x) => x.tipo === "fantasma")
										? "border-primary bg-primary/15"
										: "bg-muted/40",
								)}
							>
								<Icono icon="game-icons:ghost" className="size-4 shrink-0 opacity-80" />
								<span className="min-w-0 flex-1 truncate font-medium">{t("Fantasma")}</span>
								<Valor
									valor={valorDe({ tipo: "fantasma" }, cuenta)}
									pendiente={pendienteDe({ tipo: "fantasma" })}
								/>
							</button>
						)}
					</div>
				</ScrollArea>
				<div className="flex min-w-0 flex-1 items-center justify-center gap-4">
					<BotonSumar signo={-1} alSumar={aplicar} />
					<div className="flex min-w-24 flex-col items-center text-center leading-tight">
						{solo ? (
							<>
								<span className="max-w-40 truncate text-xs text-muted-foreground">
									{nombreDe(solo)}
								</span>
								<span className="text-6xl font-bold tabular-nums">
									{valorDe(solo, cuenta) + pendienteDe(solo)}
								</span>
								{pendienteDe(solo) !== 0 && (
									<span className="text-xs text-muted-foreground tabular-nums">
										{valorDe(solo, cuenta)} {pendienteDe(solo) > 0 ? "+" : "−"}{" "}
										{Math.abs(pendienteDe(solo))}
									</span>
								)}
							</>
						) : (
							<>
								<span className="text-xs text-muted-foreground">
									{t("{n} jugadores elegidos", { n: elegidos.length })}
								</span>
								<Icono icon="lucide:users" className="size-10 text-muted-foreground" />
							</>
						)}
					</div>
					<BotonSumar signo={1} alSumar={aplicar} />
				</div>
			</div>
		</DialogoAdaptable>
	);
}

/** El número de un jugador en la lista, con lo que va sin mandar. */
function Valor({ valor, pendiente }: { valor: number; pendiente: number }) {
	return (
		<span className="flex flex-col items-end leading-none tabular-nums">
			<b className="text-base">{valor + pendiente}</b>
			{pendiente !== 0 && (
				<span className="text-[10px] text-muted-foreground">
					{pendiente > 0 ? "+" : "−"}
					{Math.abs(pendiente)}
				</span>
			)}
		</span>
	);
}

/**
 * Un botón de − o +: un toque suma 1, mantenido suma 10 y repite mientras siga
 * pulsado. El clic que llega al soltar no hace nada más (todo va por los
 * eventos del puntero, que no se pierden aunque el dedo se salga del botón).
 */
function BotonSumar({ signo, alSumar }: { signo: 1 | -1; alSumar: (delta: number) => void }) {
	const { t } = useLang();
	const ultimo = useRef(alSumar);
	ultimo.current = alSumar;
	const mantener = useRef<Mantener | null>(null);
	if (mantener.current === null) {
		mantener.current = crearMantener(() => ultimo.current(signo * PASO_LARGO));
	}
	useEffect(() => () => mantener.current?.cancelar(), []);
	return (
		<button
			type="button"
			aria-label={signo > 0 ? t("Sumar") : t("Restar")}
			onPointerDown={(e) => {
				e.currentTarget.setPointerCapture(e.pointerId);
				mantener.current?.bajar();
			}}
			onPointerUp={() => {
				if (mantener.current?.subir()) ultimo.current(signo * PASO_TOQUE);
			}}
			onPointerCancel={() => mantener.current?.cancelar()}
			// táctil: mantener el dedo no abre el menú del sistema (sumar de diez en diez va por el puntero).
			onContextMenu={(e) => e.preventDefault()}
			className="flex size-24 shrink-0 touch-none items-center justify-center rounded-full border-2 bg-muted/40 text-5xl leading-none font-light select-none active:bg-primary/20"
		>
			{signo > 0 ? "+" : "−"}
		</button>
	);
}
