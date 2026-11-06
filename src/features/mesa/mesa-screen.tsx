import { useEffect, useRef, useState } from "react";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { registrarAtras } from "@/lib/atras";
import { call } from "@/lib/ipc";
import { abrirDeck } from "@/stores/deck-store";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { useRepeticion } from "@/stores/repeticion-store";
import { useUi } from "@/stores/ui-store";
import { hacer } from "./acciones";
import { BarraMesa } from "./barra-mesa";
import { BocadillosFlotantes, useAtajosComunicacion } from "./chat";
import { guardar, salir, suscribir, suscrito } from "./conexion";
import { CapaDados } from "./dados/capa";
import { DanoDeQuien } from "./dano-de-quien";
import { DialogosMesa } from "./dialogos";
import { usePurgarVivo } from "./en-vivo/fantasmas";
import { EntradaMesa } from "./entrada";
import { FinDePartida } from "./fin-partida";
import { useMesaMovil } from "./formato-mesa";
import { ContadorFps, VigiaFps } from "./fps";
import { crearFiltroClicTrasMenu } from "./gestos";
import { hayEncima } from "./leer-en-grande";
import { Mesas } from "./mesas";
import { useSeguirTurno } from "./mirar";
import { BarraAsientos } from "./movil/barra-asientos";
import { AvisoGirar, BotonPasarTurno } from "./movil/controles";
import { ManoCompleta } from "./movil/mano-completa";
import { PantallaVida } from "./movil/pantalla-vida";
import { Muertes } from "./muerte";
import { precargarMuestras } from "./muestras";
import { Mulligan } from "./mulligan";
import { useOriginalesAlDia } from "./originales";
import { usePantalla } from "./pantalla";
import { Ausentes } from "./red/ausentes";
import { escucharVuelta, refrescarRed } from "./red/conexion-red";
import { AvisoConexion, SelectorTablero } from "./red/estado-red";
import { PanelLateral } from "./registro-panel";
import { BarraRepeticion, salirDeLaRepeticion } from "./repeticion";
import { Ruleta } from "./ruleta/ruleta";
import { ReaccionPendiente } from "./sparring/reaccion";
import { VistaFantasma } from "./sparring/vista-fantasma";
import { AvisoTurno } from "./turno";
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

	// Las muestras de sonido se cargan al abrir la pestaña, no al primer golpe.
	useEffect(() => {
		try {
			precargarMuestras();
		} catch {
			// Sin dispositivo de audio no hay muestras y suena la sintetizada.
		}
	}, []);

	useEffect(() => {
		let vivo = true;
		void refrescarRed();
		void escucharVuelta();
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
	return (
		<>
			{activa ? <Mesa deckIds={deckIds} /> : <EntradaMesa />}
			<Ruleta />
		</>
	);
}

function Mesa({ deckIds }: { deckIds: string[] }) {
	const { t } = useLang();
	const papel = useRed((s) => s.papel);
	const repitiendo = useRepeticion((s) => s.activa !== null);
	useOriginalesAlDia();
	usePurgarVivo();
	useSeguirTurno();
	useAtajosComunicacion();
	const enMovil = useMesaMovil();
	const [preguntaSalir, setPreguntaSalir] = useState(false);
	const [preguntaReiniciar, setPreguntaReiniciar] = useState(false);
	const [preguntaEditar, setPreguntaEditar] = useState(false);
	// Siendo anfitrión: si al salir la partida sigue con otro jugador.
	const [puedeCeder, setPuedeCeder] = useState(false);
	// Los decks de la partida, por asiento (el estado se leyó antes de empezar y puede venir vacío).
	const [ids, setIds] = useState(deckIds);
	useEffect(() => {
		if (deckIds.length > 0) return setIds(deckIds);
		void call("mesa_estado").then((e) => setIds(e.deckIds));
	}, [deckIds]);
	// El deck de quien juega ahora (para editarlo y para su tapete).
	const asientos = useMesa((s) => s.asientos);
	const yoReal = useMesa((s) => s.yo);
	// En red solo se sabe el propio (el de los demás llega vacío): el de mi asiento.
	const deckId2 = ids[Math.max(0, asientos.indexOf(yoReal ?? -1))] || ids[0] || null;

	// El botón «atrás» de Android: lo de arriba (diálogos, menús, la mano, la vida, el
	// registro) lo cierra `lib/atras.ts` solo, por ser capas de Radix; antes de salir se
	// quita lo que se está leyendo o eligiendo, y por fin se pregunta si salir de la
	// partida (queda guardada), como el botón de la barra.
	const salirAlPulsarAtras = () => {
		if (repitiendo) return void salirDeLaRepeticion();
		if (papel === "anfitrion") {
			call("red_estado")
				.then((e) => setPuedeCeder(e.puedeCeder))
				.catch(() => setPuedeCeder(false));
		}
		setPreguntaSalir(true);
	};
	const salirRef = useRef(salirAlPulsarAtras);
	salirRef.current = salirAlPulsarAtras;
	useEffect(
		() =>
			registrarAtras(() => {
				const m = useMesa.getState();
				if (m.sobre !== null) {
					m.setSobre(null);
					return true;
				}
				const p = usePantalla.getState();
				if (p.fantasma !== null) {
					p.setFantasma(null);
					return true;
				}
				if (p.modo) {
					p.setModo(null);
					return true;
				}
				salirRef.current();
				return true;
			}),
		[],
	);

	// Mantener el dedo sobre algo que también hace algo al tocarlo abre su menú; el clic que
	// llega al soltar no se quiere (robaría una carta, cambiaría de tablero…).
	useEffect(() => {
		const filtro = crearFiltroClicTrasMenu();
		// Con el ratón no hay clic tras el clic derecho: solo se filtra lo del dedo.
		let conDedo = false;
		const alMenu = () => {
			if (!conDedo) return;
			filtro.menu();
			// La carta que se leía en grande taparía el menú.
			useMesa.getState().setSobre(null);
			usePantalla.getState().setFantasma(null);
		};
		const alBajar = (e: PointerEvent) => {
			conDedo = !hayEncima(e.pointerType);
			filtro.bajar();
		};
		const alClic = (e: MouseEvent) => {
			if (!filtro.clic()) return;
			e.stopPropagation();
			e.preventDefault();
		};
		// táctil: solo se oye cuándo sale un menú (clic derecho o pulsación larga) para descartar el clic de al soltar.
		window.addEventListener("contextmenu", alMenu, true);
		window.addEventListener("pointerdown", alBajar, true);
		window.addEventListener("click", alClic, true);
		return () => {
			window.removeEventListener("contextmenu", alMenu, true);
			window.removeEventListener("pointerdown", alBajar, true);
			window.removeEventListener("click", alClic, true);
		};
	}, []);

	const editarDeck = async () => {
		if (!deckId2) return;
		await guardar();
		await abrirDeck(deckId2);
		useUi.getState().setVista("deck");
	};

	return (
		<div className="relative isolate size-full overflow-hidden bg-black">
			<Mesas deckIds={ids} />
			{repitiendo ? (
				<BarraRepeticion />
			) : (
				<BarraMesa
					onSalir={() => {
						if (papel === "anfitrion") {
							call("red_estado")
								.then((e) => setPuedeCeder(e.puedeCeder))
								.catch(() => setPuedeCeder(false));
						}
						setPreguntaSalir(true);
					}}
					onEditarDeck={() => setPreguntaEditar(true)}
					onReiniciar={() => setPreguntaReiniciar(true)}
				/>
			)}
			<SelectorTablero />
			{enMovil && !repitiendo && (
				<>
					<BarraAsientos />
					<BotonPasarTurno />
					<ManoCompleta />
					<PantallaVida />
				</>
			)}
			{enMovil && <AvisoGirar />}
			<AvisoConexion />
			{!repitiendo && <Ausentes />}
			<CapaDados />
			{!repitiendo && <ReaccionPendiente />}
			<BocadillosFlotantes />
			<PanelLateral />
			<VistaGrande />
			<VistaFantasma />
			{!repitiendo && (
				<>
					<AvisoTurno />
					<Mulligan />
					<DanoDeQuien />
					<Muertes />
					<FinDePartida />
				</>
			)}
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
							? puedeCeder
								? t("Eres el anfitrión: la partida sigue con otro jugador.")
								: t("Eres el anfitrión: la partida se cierra para todos.")
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
