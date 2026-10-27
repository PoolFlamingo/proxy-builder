import { Flag, LogOut, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { call } from "@/lib/ipc";
import { abrirDeck } from "@/stores/deck-store";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { hacer } from "./acciones";
import { BanquilloDialog } from "./banquillo-dialog";
import { BarraMesa } from "./barra-mesa";
import { guardar, salir, suscribir, suscrito } from "./conexion";
import { CapaDados } from "./dados/capa";
import { DanoDeQuien } from "./dano-de-quien";
import { DialogosMesa } from "./dialogos";
import { COMO } from "./dialogos/terminar";
import { useEncuentro } from "./encuentro";
import { EntradaMesa } from "./entrada";
import { ContadorFps, VigiaFps } from "./fps";
import { Mesas } from "./mesas";
import { useSeguirTurno } from "./mirar";
import { Muertes } from "./muerte";
import { Mulligan } from "./mulligan";
import { useOriginalesAlDia } from "./originales";
import { Ausentes } from "./red/ausentes";
import { refrescarRed } from "./red/conexion-red";
import { AvisoConexion, SelectorTablero } from "./red/estado-red";
import { ReaccionPendiente } from "./sparring/reaccion";
import { VistaGrande } from "./vista-grande";

/**
 * **La pestaña «Jugar»**: la entrada (elegir deck y opciones) o, con una
 * partida abierta, la mesa. Al salir de la pestaña la partida sigue abierta
 * (y se guarda); al volver, se retoma tal cual.
 */
export function MesaScreen() {
	const activa = useMesa((s) => s.activa);
	const [comprobado, setComprobado] = useState(false);
	const [deckIds, setDeckIds] = useState<string[]>([]);

	useEffect(() => {
		let vivo = true;
		void refrescarRed();
		call("mesa_estado")
			.then(async (e) => {
				setDeckIds(e.deckIds);
				if (e.abierta && !useMesa.getState().activa && !suscrito()) await suscribir();
			})
			.catch(() => {})
			.finally(() => vivo && setComprobado(true));
		return () => {
			vivo = false;
			void guardar();
		};
	}, []);

	if (!comprobado) return null;
	if (!activa) return <EntradaMesa />;
	return <Mesa deckId={deckIds[0] ?? null} />;
}

function Mesa({ deckId }: { deckId: string | null }) {
	const { t } = useLang();
	const papel = useRed((s) => s.papel);
	useOriginalesAlDia();
	useSeguirTurno();
	const [preguntaSalir, setPreguntaSalir] = useState(false);
	const [preguntaReiniciar, setPreguntaReiniciar] = useState(false);
	const [preguntaEditar, setPreguntaEditar] = useState(false);
	const [deckId2, setDeckId2] = useState(deckId);
	useEffect(() => {
		if (deckId) return setDeckId2(deckId);
		void call("mesa_estado").then((e) => setDeckId2(e.deckIds[0] ?? null));
	}, [deckId]);

	const editarDeck = async () => {
		if (!deckId2) return;
		await guardar();
		await abrirDeck(deckId2);
		useUi.getState().setVista("deck");
	};

	return (
		<div className="relative isolate size-full overflow-hidden bg-black">
			<Mesas deckId={deckId2} />
			<BarraMesa
				onSalir={() => setPreguntaSalir(true)}
				onEditarDeck={() => setPreguntaEditar(true)}
				onReiniciar={() => setPreguntaReiniciar(true)}
			/>
			<SelectorTablero />
			<AvisoConexion />
			<Ausentes />
			<CapaDados />
			<ReaccionPendiente />
			<VistaGrande />
			<Mulligan />
			<DanoDeQuien />
			<Muertes />
			<FinDePartida />
			<DialogosMesa deckId={deckId2} />
			<ContadorFps />
			<VigiaFps />
			{papel ? (
				<Confirmar
					abierto={preguntaSalir}
					onCerrar={() => setPreguntaSalir(false)}
					titulo={t("¿Salir de la partida?")}
					descripcion={
						papel === "anfitrion"
							? t("Eres el anfitrión: la partida se cierra para todos.")
							: t("Los demás siguen jugando sin ti.")
					}
					accion={t("Salir")}
					destructiva
					onConfirmar={() => void salir(false)}
				/>
			) : (
				<Confirmar
					abierto={preguntaSalir}
					onCerrar={() => setPreguntaSalir(false)}
					titulo={t("¿Salir de la mesa?")}
					descripcion={t("Puedes dejar la partida guardada para seguirla luego o terminarla.")}
					accion={t("Terminarla")}
					destructiva
					onConfirmar={() => void salir(false)}
					accionAlternativa={{
						texto: t("Dejarla para luego"),
						onClick: async () => {
							await salir(true);
							return true;
						},
					}}
				/>
			)}
			<Confirmar
				abierto={preguntaReiniciar}
				onCerrar={() => setPreguntaReiniciar(false)}
				titulo={t("¿Empezar otra partida?")}
				descripcion={t("Con el mismo deck y una mano nueva. La partida de ahora se pierde.")}
				accion={t("Otra partida")}
				onConfirmar={() => void hacer({ que: "reiniciar" })}
			/>
			<Confirmar
				abierto={preguntaEditar}
				onCerrar={() => setPreguntaEditar(false)}
				titulo={t("¿Editar este deck?")}
				descripcion={t("La partida se queda guardada: la sigues al volver a «Jugar».")}
				accion={t("Editar el deck")}
				onConfirmar={() => void editarDeck()}
			/>
		</div>
	);
}

/** Al terminar: quién ganó y cómo, otra partida o salir (`capturas/pablo/21`). */
function FinDePartida() {
	const { t } = useLang();
	const fin = useMesa((s) => s.general?.fin ?? null);
	const jugadores = useMesa((s) => s.jugadores);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const encuentro = useEncuentro();
	const [oculto, setOculto] = useState(false);
	const [banquillo, setBanquillo] = useState(false);
	useEffect(() => {
		if (fin) setOculto(false);
	}, [fin]);
	if (!fin || (oculto && !banquillo)) return null;
	const nombre = (g: number) => jugadores[g]?.nombre ?? "?";
	const ganadores = fin.ganadores.map(nombre);
	const como = COMO.find((c) => c.valor === fin.como)?.nombre ?? "";
	const estado = encuentro?.estado ?? null;
	const decide = estado?.terminado ?? false;
	return (
		<div className="absolute inset-0 z-[8800] flex items-center justify-center bg-black/60 backdrop-blur-sm">
			<div className="flex w-96 flex-col items-center gap-2.5 rounded-xl border bg-card p-4 text-center shadow-2xl">
				<Flag className="size-6 text-primary" />
				<h2 className="text-base font-semibold">
					{ganadores.length === 0
						? asientos.length > 1
							? t("Empate")
							: t("Ganó el rival")
						: t("Ganó {quien}", { quien: ganadores.join(", ") })}
				</h2>
				<p className="text-xs text-muted-foreground">{t(como)}</p>
				{fin.nota && <p className="text-xs">{fin.nota}</p>}
				{estado && (
					<div className="flex w-full flex-col gap-1 rounded-lg border bg-background/40 p-2 text-xs">
						<span className="text-muted-foreground">
							{t("Al mejor de {total}: partida {actual}", {
								total: estado.alMejorDe,
								actual: estado.jugadas,
							})}
						</span>
						<span className="flex flex-wrap justify-center gap-x-4 gap-y-0.5 tabular-nums">
							{asientos.map((id) => (
								<span key={id}>
									{nombre(id)}: <b>{estado.marcador[id] ?? 0}</b>
								</span>
							))}
						</span>
						{decide && (
							<b className="text-primary">
								{estado.ganadores.length === 0
									? t("El encuentro acaba en tablas")
									: t("Gana el encuentro {quien}", {
											quien: estado.ganadores.map(nombre).join(", "),
										})}
							</b>
						)}
					</div>
				)}
				<div className="flex gap-2.5">
					<Button variant="outline" onClick={() => setOculto(true)}>
						{t("Ver la mesa")}
					</Button>
					<Button variant="outline" onClick={() => void salir(false)}>
						<LogOut className="size-4" />
						{t("Salir")}
					</Button>
					{estado ? (
						!decide && (
							<Button onClick={() => setBanquillo(true)}>
								<RotateCcw className="size-4" />
								{t("Siguiente partida")}
							</Button>
						)
					) : (
						<Button onClick={() => void hacer({ que: "reiniciar" })}>
							<RotateCcw className="size-4" />
							{t("Otra partida")}
						</Button>
					)}
				</div>
			</div>
			{banquillo && <BanquilloDialog onCerrar={() => setBanquillo(false)} />}
		</div>
	);
}
