import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { useShallow } from "zustand/react/shallow";
import type { Causa } from "@/bindings/Causa";
import type { IdJugador } from "@/bindings/IdJugador";
import { Selector } from "@/components/campos";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { hacer } from "./acciones";
import { ganadoresEnPie } from "./cuentas-fin";
import { claveCausa, ESPERA_APUNTAR_MS, eliminadoPorDefecto } from "./cuentas-muerte";
import { useDanoDe } from "./dano-de-quien";
import { usePantalla } from "./pantalla";

const NADIE = "nadie";

/** El motivo, con la cifra que lo causa (la que ve la mesa del jugador). */
function useMotivo() {
	const { t } = useLang();
	const jugadores = useMesa((s) => s.jugadores);
	return (id: IdJugador, c: Causa): string => {
		const j = jugadores[id];
		switch (c.tipo) {
			case "vida":
				return t("Vida a 0 o menos.");
			case "veneno":
				return t("Veneno: {cuantos}", { cuantos: j?.veneno ?? 0 });
			case "comandante":
				return t("Daño de comandante: {cuantos}", {
					cuantos: j?.danoComandante[c.carta] ?? 0,
				});
		}
	};
}

/**
 * **La muerte, confirmada por quien muere** (G8). Rust avisa de que alguien
 * llegó a 0 vidas, al límite de veneno o al de un comandante
 * (`VistaJugador.fueraDeJuego`) y esa persona confirma «¿Estás fuera?» (y por
 * quién). En una partida local, quien juega confirma por cada asiento. Para
 * los demás, en red, es un aviso que no bloquea: pasados 15 s sin respuesta,
 * cualquiera puede apuntar su muerte. Con la ayuda «Avisar a 0 vidas» apagada,
 * no hay nada de esto.
 */
export function Muertes() {
	const avisar = useMesa((s) => s.opciones?.ayudas.avisarVidaCero ?? false);
	const fin = useMesa((s) => s.general?.fin != null);
	const espectador = useRed((s) => s.papel === "espectador");
	if (!avisar || fin || espectador) return null;
	return (
		<>
			<ConfirmarMuerte />
			<AvisosAjenos />
			<SoloQuedaUno />
		</>
	);
}

function useControlados(): IdJugador[] {
	const asientos = useMesa(useShallow((s) => s.asientos));
	const yo = useMesa((s) => s.yo ?? 0);
	const enRed = useRed((s) => s.papel !== null);
	return enRed ? asientos.filter((id) => id === yo) : asientos;
}

function ConfirmarMuerte() {
	const { t } = useLang();
	const motivoDe = useMotivo();
	const controlados = useControlados();
	const jugadores = useMesa((s) => s.jugadores);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const activo = useMesa((s) => s.general?.turno.activo ?? 0);
	const yo = useMesa((s) => s.yo ?? 0);
	const ultimo = useDanoDe((s) => s.ultimo);
	// «Sigo en juego»: no se vuelve a preguntar por lo mismo (hasta que cambie el motivo).
	const [descartados, setDescartados] = useState<Record<number, string>>({});
	const [elegido, setElegido] = useState<{ jugador: IdJugador; por: IdJugador | null } | null>(
		null,
	);

	useEffect(() => {
		setDescartados((d) => {
			let cambia = false;
			const n = { ...d };
			for (const id of Object.keys(d).map(Number)) {
				if (jugadores[id]?.fueraDeJuego == null) {
					delete n[id];
					cambia = true;
				}
			}
			return cambia ? n : d;
		});
	}, [jugadores]);

	const pendiente = controlados.find((id) => {
		const c = jugadores[id]?.fueraDeJuego;
		return c != null && jugadores[id]?.eliminado == null && descartados[id] !== claveCausa(c);
	});
	if (pendiente === undefined) return null;
	const j = jugadores[pendiente];
	const causa = j?.fueraDeJuego;
	if (!j || !causa) return null;
	const vivos = asientos.filter((id) => jugadores[id]?.eliminado == null);
	const defecto = eliminadoPorDefecto({
		causa,
		jugador: pendiente,
		activo,
		ultimoDano: ultimo[pendiente],
		vivos,
	});
	const por = elegido?.jugador === pendiente ? elegido.por : defecto;
	const otros = vivos.filter((id) => id !== pendiente);
	const soyYo = pendiente === yo && useRed.getState().papel !== null;
	return (
		<Dialog
			open
			onOpenChange={(o) => {
				if (!o) setDescartados((d) => ({ ...d, [pendiente]: claveCausa(causa) }));
			}}
		>
			<DialogContent className="sm:max-w-sm" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<Icono icon="game-icons:death-skull" className="size-4 text-rose-400" />
						{soyYo
							? t("¿Confirmas que estás fuera?")
							: t("¿Confirmas que {quien} está fuera?", { quien: j.nombre })}
					</DialogTitle>
					<DialogDescription>{motivoDe(pendiente, causa)}</DialogDescription>
				</DialogHeader>
				{otros.length > 0 && (
					<div className="flex items-center justify-between gap-2.5 text-xs">
						<span>{t("Eliminado por")}</span>
						<Selector<string>
							valor={por === null ? NADIE : String(por)}
							onChange={(v) =>
								setElegido({ jugador: pendiente, por: v === NADIE ? null : Number(v) })
							}
							opciones={[
								{ valor: NADIE, etiqueta: t("Nadie en concreto") },
								...otros.map((id) => ({
									valor: String(id),
									etiqueta: jugadores[id]?.nombre ?? "",
								})),
							]}
							ancho="w-44"
						/>
					</div>
				)}
				<DialogFooter>
					<Button
						variant="outline"
						onClick={() => setDescartados((d) => ({ ...d, [pendiente]: claveCausa(causa) }))}
					>
						{t("Sigo en juego")}
					</Button>
					<Button
						variant="destructive"
						onClick={() => void hacer({ que: "eliminar", jugador: pendiente, por })}
					>
						<Icono icon="game-icons:death-skull" className="size-4" />
						{soyYo ? t("Estoy fuera") : t("Está fuera")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

/**
 * En red, los avisos de los otros: al llegar un jugador a un límite sale un
 * aviso y, a los 15 s sin que él lo confirme, el aviso ofrece apuntarlo.
 */
function AvisosAjenos() {
	const { t } = useLang();
	const motivoDe = useMotivo();
	const enRed = useRed((s) => s.papel !== null);
	const yo = useMesa((s) => s.yo ?? 0);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const activo = useMesa((s) => s.general?.turno.activo ?? 0);
	const relojes = useRef<Map<number, number>>(new Map());
	const avisados = useRef<Map<number, string>>(new Map());

	useEffect(() => {
		if (!enRed) return;
		const vivos = asientos.filter((id) => jugadores[id]?.eliminado == null);
		for (const id of asientos) {
			if (id === yo) continue;
			const j = jugadores[id];
			const c = j?.fueraDeJuego;
			const clave = c && j?.eliminado == null ? claveCausa(c) : null;
			if (clave === null || !c || !j) {
				if (avisados.current.has(id)) {
					avisados.current.delete(id);
					toast.dismiss(`muerte-${id}`);
					const r = relojes.current.get(id);
					if (r !== undefined) window.clearTimeout(r);
					relojes.current.delete(id);
				}
				continue;
			}
			if (avisados.current.get(id) === clave) continue;
			avisados.current.set(id, clave);
			toast.warning(
				t("{quien} ha llegado a un límite. {motivo}", {
					quien: j.nombre,
					motivo: motivoDe(id, c),
				}),
				{ id: `muerte-${id}`, duration: ESPERA_APUNTAR_MS },
			);
			const viejo = relojes.current.get(id);
			if (viejo !== undefined) window.clearTimeout(viejo);
			relojes.current.set(
				id,
				window.setTimeout(() => {
					const m = useMesa.getState();
					const por = eliminadoPorDefecto({
						causa: c,
						jugador: id,
						activo: m.general?.turno.activo ?? activo,
						ultimoDano: undefined,
						vivos,
					});
					toast.warning(
						t("{quien} no ha confirmado si está fuera. ¿Lo apuntas?", { quien: j.nombre }),
						{
							id: `muerte-${id}`,
							duration: Number.POSITIVE_INFINITY,
							action: {
								label: t("Apuntar que está fuera"),
								onClick: () => void hacer({ que: "eliminar", jugador: id, por }),
							},
						},
					);
				}, ESPERA_APUNTAR_MS),
			);
		}
	}, [enRed, yo, asientos, jugadores, activo, t, motivoDe]);

	useEffect(
		() => () => {
			for (const r of relojes.current.values()) window.clearTimeout(r);
			relojes.current.clear();
		},
		[],
	);
	return null;
}

/**
 * Cuando solo queda uno en pie (o un equipo), propone terminar la partida con
 * él ya marcado como ganador. Una vez por cada vez que ocurre; si se deshace
 * la muerte, el aviso se va.
 */
function SoloQuedaUno() {
	const { t } = useLang();
	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const avisado = useRef<string>("");
	useEffect(() => {
		const gana = ganadoresEnPie(asientos, (id) => {
			const j = jugadores[id];
			return j ? { eliminado: j.eliminado != null, equipo: j.equipo } : undefined;
		});
		const clave = gana.join(",");
		if (clave === avisado.current) return;
		avisado.current = clave;
		if (gana.length === 0) {
			toast.dismiss("solo-queda");
			return;
		}
		toast(
			t("Solo queda {quien} en pie. ¿Terminas la partida?", {
				quien: gana.map((id) => jugadores[id]?.nombre ?? "?").join(", "),
			}),
			{
				id: "solo-queda",
				duration: Number.POSITIVE_INFINITY,
				action: {
					label: t("Terminar la partida"),
					onClick: () => usePantalla.getState().abrir({ tipo: "terminar" }),
				},
				cancel: { label: t("Seguir jugando"), onClick: () => {} },
			},
		);
	}, [asientos, jugadores, t]);
	useEffect(() => () => void toast.dismiss("solo-queda"), []);
	return null;
}
