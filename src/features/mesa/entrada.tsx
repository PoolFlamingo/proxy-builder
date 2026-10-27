import {
	History,
	Image,
	ListChecks,
	Loader2,
	Pencil,
	Play,
	Plug,
	Plus,
	Trash2,
	User,
	Users,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { EstadoMesa } from "@/bindings/EstadoMesa";
import type { Formato } from "@/bindings/Formato";
import type { Intensidad } from "@/bindings/Intensidad";
import type { ListaDeJuego } from "@/bindings/ListaDeJuego";
import type { ListasDeDeck } from "@/bindings/ListasDeDeck";
import type { ModoMulligan } from "@/bindings/ModoMulligan";
import type { Opciones } from "@/bindings/Opciones";
import { CampoNumero, Fila, FilaInterruptor, Selector } from "@/components/campos";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { BarraHerramientas } from "@/features/library/barra-herramientas";
import { BarraLateral } from "@/features/library/barra-lateral";
import { useEtiquetasUi } from "@/features/library/etiquetas-ui";
import { type Acciones, AccionesDeck } from "@/features/library/menu-deck";
import {
	ORGANIZADOR_JUGAR,
	OrganizadorContexto,
	usePrefsJugar,
} from "@/features/library/preferencias";
import { PropiedadesDialog } from "@/features/library/propiedades-dialog";
import { useBiblioteca } from "@/features/library/store";
import { type Seleccion, SeleccionDecks } from "@/features/library/vista-cuadricula";
import { useOrganizados, ZonaDecks } from "@/features/library/zona-decks";
import { call } from "@/lib/ipc";
import { abrirDeck } from "@/stores/deck-store";
import { useRed } from "@/stores/red-store";
import { useSalas } from "@/stores/salas-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";
import { SalasDialog } from "../salas/salas-dialog";
import { COLORES_JUGADOR } from "./colores";
import { empezar, retomar, salir } from "./conexion";
import { HistorialGeneralDialog } from "./historial-general";
import { ListaDialog } from "./listas/lista-dialog";
import { opcionesDeFabrica } from "./opciones-fabrica";
import { PartidaLocalDialog } from "./partida-local";
import { RedDialog } from "./red/red-dialog";
import { INTENSIDADES } from "./sparring/textos";
import { TapeteDialog } from "./tapete-dialog";

const CLAVE_NOMBRE = "proxybuilder.mesa.nombre";

function nombreGuardado(): string {
	try {
		return localStorage.getItem(CLAVE_NOMBRE) ?? "";
	} catch {
		return "";
	}
}

/**
 * **La entrada a la mesa** (pestaña «Jugar»): elegir un deck de la
 * biblioteca, el formato y las opciones, y empezar en solitario; o seguir la
 * partida que quedó a medias.
 */
/** El valor del selector de listas para jugar con el deck entero (Radix no admite «»). */
const TODO_EL_DECK = "*";

export function EntradaMesa() {
	return (
		<OrganizadorContexto.Provider value={ORGANIZADOR_JUGAR}>
			<Entrada />
		</OrganizadorContexto.Provider>
	);
}

const SIN_DECKS: never[] = [];

function Entrada() {
	const { t, mensaje } = useLang();
	const decks = useBiblioteca((s) => s.decks) ?? SIN_DECKS;
	const prefs = usePrefsJugar();
	const org = useOrganizados();
	const etiquetasUi = useEtiquetasUi();
	const buscar = useRef<HTMLInputElement>(null);
	const [propiedades, setPropiedades] = useState<string | null>(null);
	const [estado, setEstado] = useState<EstadoMesa | null>(null);
	const [elegido, setElegido] = useState<string | null>(null);
	const sparring = useUi((s) => s.mesa.sparring);
	const [formato, setFormato] = useState<Formato>("commander");
	const [opciones, setOpciones] = useState<Opciones>(() => opcionesDeFabrica("commander", 1));
	const [nombre, setNombre] = useState(nombreGuardado);
	const [tapete, setTapete] = useState(false);
	const [descartar, setDescartar] = useState(false);
	const [reemplazar, setReemplazar] = useState(false);
	// Las listas para jugar del deck elegido y la que se usa.
	const [listas, setListas] = useState<ListasDeDeck | null>(null);
	const [listaId, setListaId] = useState<string | null>(null);
	const [editando, setEditando] = useState<{ lista: ListaDeJuego | null } | null>(null);
	const [red, setRed] = useState(false);
	const [amigos, setAmigos] = useState(false);
	const [local, setLocal] = useState(false);
	const [historial, setHistorial] = useState(false);
	const enSalaServidor = useSalas((s) => s.sala !== null);
	const enlaceEntrante = useSalas((s) => s.enlaceEntrante);
	// Un enlace de invitación abre «Con amigos…» con el código puesto.
	useEffect(() => {
		if (enlaceEntrante) setAmigos(true);
	}, [enlaceEntrante]);
	const enSala = useRed((s) => s.papel !== null);
	const trabajo = useTrabajo((s) => (s.actual?.tipo === "mesa" ? s.actual : null));

	useEffect(() => {
		useBiblioteca
			.getState()
			.cargar()
			.catch((e) => toast.error(mensaje(e)));
		call("mesa_estado")
			.then(setEstado)
			.catch(() => setEstado(null));
	}, [mensaje]);

	// Sin deck elegido, el que se abrió hace menos.
	useEffect(() => {
		if (decks.length === 0) return;
		const reciente = [...decks].sort((a, b) => (b.openedAt ?? 0) - (a.openedAt ?? 0))[0];
		if (reciente) setElegido((e) => (e && decks.some((d) => d.id === e) ? e : reciente.id));
	}, [decks]);

	useEffect(() => {
		setListas(null);
		setListaId(null);
		if (!elegido) return;
		call("mesa_listas", { deck: elegido })
			.then((l) => {
				setListas(l);
				setListaId(l.listas[0]?.id ?? null);
			})
			.catch(() => setListas(null));
	}, [elegido]);
	/** El deck pasa del tamaño del formato: hay que elegir una lista (como `Construccion::pide_lista`). */
	const pideLista = formato === "commander" && (listas?.cuentan ?? 0) > 100;
	const sinLista = pideLista && !listaId;

	// El formato de fábrica, el que se deduce del deck.
	useEffect(() => {
		if (!elegido) return;
		call("deck_details", { ids: [elegido] })
			.then(([d]) => {
				if (!d) return;
				const f: Formato =
					d.kind === "commander" ? "commander" : d.kind === "limited" ? "limited" : "constructed";
				setFormato(f);
				setOpciones(opcionesDeFabrica(f, 1));
			})
			.catch(() => {});
	}, [elegido]);

	/** Empezar; con una partida a medias, antes se pregunta (se perdería). */
	const pedirSolitario = () => {
		if (!elegido) return;
		if (estado?.guardada) setReemplazar(true);
		else void solitario();
	};
	const solitario = async () => {
		if (!elegido) return;
		await empezar({
			jugadores: [
				{
					deckId: elegido,
					nombre: nombre.trim() || t("Jugador"),
					color: COLORES_JUGADOR[0],
					lista: listaId ?? undefined,
				},
			],
			formato,
			variantes: {
				equipos: { tipo: "ninguno" },
				archenemy: null,
				planechase: false,
				manoPrivadaEnEquipo: false,
			},
			opciones,
			intensidad: sparring,
			equipos: [],
		});
	};

	const cambiarFormato = (f: Formato) => {
		setFormato(f);
		setOpciones(opcionesDeFabrica(f, 1));
	};
	const ayuda = (k: keyof Opciones["ayudas"], v: boolean) =>
		setOpciones({ ...opciones, ayudas: { ...opciones.ayudas, [k]: v } });

	// Ctrl+F busca, como en la biblioteca.
	useEffect(() => {
		const tecla = (e: KeyboardEvent) => {
			if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "f") return;
			if ((e.target as HTMLElement | null)?.closest('[role="dialog"],[role="alertdialog"]')) return;
			e.preventDefault();
			buscar.current?.focus();
			buscar.current?.select();
		};
		window.addEventListener("keydown", tecla);
		return () => window.removeEventListener("keydown", tecla);
	}, []);

	// Pulsar un deck lo elige; el doble clic empieza en solitario. Sin casillas
	// ni acciones que borren: aquí se viene a jugar.
	const sel: Seleccion = {
		seleccion: [],
		casillas: false,
		abierto: elegido,
		pulsar: setElegido,
		alternar: () => {},
		marcar: () => {},
		abrir: setElegido,
		alDobleClic: (id) => {
			if (id !== elegido) setElegido(id);
			else if (!trabajo && !sinLista) pedirSolitario();
		},
	};
	const acciones: Acciones = {
		abrir: (id) => {
			abrirDeck(id)
				.then(() => useUi.getState().setVista("deck"))
				.catch((e) => toast.error(mensaje(e)));
		},
		propiedades: setPropiedades,
		nuevaEtiqueta: (ids) => etiquetasUi.editar({ tipo: "nueva", para: ids }),
		seleccion: [],
	};

	const guardada = estado?.guardada;
	/** Lo que la red y las salas necesitan de la entrada: el deck elegido y sus opciones. */
	const preparar = {
		deckId: elegido,
		deckNombre: decks.find((d) => d.id === elegido)?.name ?? null,
		lista: listaId,
		sinLista,
		formato,
		opciones,
		nombre: nombre.trim() || t("Jugador"),
	};
	return (
		<AccionesDeck.Provider value={acciones}>
			<SeleccionDecks.Provider value={sel}>
				<div className="flex h-full min-h-0 gap-2.5 p-2.5" data-tutorial="mesa-entrada">
					<section className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border bg-card/40">
						<BarraHerramientas
							ref={buscar}
							titulo={t("Elige un deck")}
							acciones={
								<Button size="sm" variant="outline" onClick={() => setHistorial(true)}>
									<History />
									{t("Historial")}
								</Button>
							}
						/>
						<div className="flex min-h-0 flex-1">
							{prefs.lateral && (
								<BarraLateral
									onNuevaEtiqueta={() => etiquetasUi.editar({ tipo: "nueva", para: [] })}
									onEditarEtiqueta={(tag) => etiquetasUi.editar({ tipo: "editar", tag })}
									onBorrarEtiqueta={etiquetasUi.borrar}
								/>
							)}
							<ZonaDecks org={org} />
						</div>
					</section>

					<aside className="flex w-80 shrink-0 flex-col gap-2.5">
						{guardada && (
							<section className="flex flex-col gap-2.5 rounded-lg border border-primary/40 bg-primary/5 p-2.5">
								<h3 className="text-xs font-semibold">{t("Partida a medias")}</h3>
								<p className="text-xs text-muted-foreground">
									{t("{deck} · turno {turno}", {
										deck: guardada.mazos.join(", "),
										turno: guardada.turno,
									})}
								</p>
								<div className="flex gap-2.5">
									<Button
										size="sm"
										className="flex-1"
										disabled={!!trabajo}
										onClick={() => void retomar()}
									>
										<Play className="size-3.5" />
										{t("Seguir la partida")}
									</Button>
									<Button size="sm" variant="outline" onClick={() => setDescartar(true)}>
										<Trash2 className="size-3.5" />
										{t("Descartar")}
									</Button>
								</div>
							</section>
						)}
						<section className="flex flex-col gap-2.5 rounded-lg border bg-card/40 p-2.5">
							<Fila etiqueta={t("Tu nombre")}>
								<div className="relative">
									<User className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
									<Input
										value={nombre}
										onChange={(e) => {
											setNombre(e.target.value);
											try {
												localStorage.setItem(CLAVE_NOMBRE, e.target.value);
											} catch {
												/* sin almacenamiento */
											}
										}}
										placeholder={t("Jugador")}
										maxLength={24}
										className="h-7 w-36 pl-7 text-xs"
									/>
								</div>
							</Fila>
							<Fila etiqueta={t("Formato")}>
								<Selector<Formato>
									valor={formato}
									onChange={cambiarFormato}
									opciones={[
										{ valor: "commander", etiqueta: "Commander" },
										{ valor: "constructed", etiqueta: t("60 cartas") },
										{ valor: "limited", etiqueta: t("Limitado") },
										{ valor: "libre", etiqueta: t("Libre") },
									]}
								/>
							</Fila>
							{elegido && listas && (
								<div className="flex flex-col gap-1.5" data-tutorial="mesa-lista">
									<Fila etiqueta={t("Lista para jugar")}>
										{pideLista && listas.listas.length === 0 ? (
											<span className="text-xs text-muted-foreground">{t("Ninguna todavía")}</span>
										) : (
											<Selector<string>
												valor={listaId ?? TODO_EL_DECK}
												onChange={(v) => setListaId(v === TODO_EL_DECK ? null : v)}
												opciones={[
													...(pideLista
														? []
														: [
																{
																	valor: TODO_EL_DECK,
																	etiqueta: t("Todo el deck ({n})", { n: listas.cuentan }),
																},
															]),
													...listas.listas.map((l) => ({ valor: l.id, etiqueta: l.nombre })),
												]}
											/>
										)}
									</Fila>
									<div className="flex justify-end gap-1.5">
										<Button
											size="sm"
											variant="outline"
											onClick={() => setEditando({ lista: null })}
										>
											<Plus className="size-3.5" />
											{t("Nueva lista…")}
										</Button>
										<Button
											size="sm"
											variant="outline"
											disabled={!listaId}
											onClick={() =>
												setEditando({ lista: listas.listas.find((l) => l.id === listaId) ?? null })
											}
										>
											<Pencil className="size-3.5" />
											{t("Editar…")}
										</Button>
									</div>
									{sinLista && (
										<p className="flex items-start gap-1.5 rounded-md bg-amber-500/10 p-1.5 text-[11px] text-amber-600 dark:text-amber-300">
											<ListChecks className="mt-0.5 size-3.5 shrink-0" />
											{t(
												"Este deck tiene {n} cartas y en Commander se juega con 100: haz una lista con el comandante y 99 más.",
												{ n: listas.cuentan },
											)}
										</p>
									)}
								</div>
							)}
							<Fila etiqueta={t("Vidas")}>
								<CampoNumero
									valor={opciones.vidaInicial}
									onChange={(v) => setOpciones({ ...opciones, vidaInicial: v ?? 20 })}
									min={1}
									max={999}
									paso={1}
								/>
							</Fila>
							<Fila etiqueta={t("Mulligan")}>
								<Selector<ModoMulligan>
									valor={opciones.mulligan}
									onChange={(m) => setOpciones({ ...opciones, mulligan: m })}
									opciones={[
										{ valor: "londres", etiqueta: t("Londres") },
										{ valor: "londresPrimeroGratis", etiqueta: t("Londres, el primero gratis") },
										{ valor: "amistoso", etiqueta: t("Amistoso") },
										{ valor: "libre", etiqueta: t("Libre") },
									]}
								/>
							</Fila>
							<FilaInterruptor
								etiqueta={t("Robar al empezar el turno")}
								valor={opciones.ayudas.robarAlEmpezar}
								onChange={(v) => ayuda("robarAlEmpezar", v)}
							/>
							<FilaInterruptor
								etiqueta={t("Enderezar al empezar el turno")}
								valor={opciones.ayudas.enderezarAlEmpezar}
								onChange={(v) => ayuda("enderezarAlEmpezar", v)}
							/>
							<FilaInterruptor
								etiqueta={t("El primero no roba")}
								valor={opciones.primeroNoRoba}
								onChange={(v) => setOpciones({ ...opciones, primeroNoRoba: v })}
							/>
							<FilaInterruptor
								etiqueta={t("Sumar el impuesto del comandante")}
								valor={opciones.ayudas.impuestoAutomatico}
								onChange={(v) => ayuda("impuestoAutomatico", v)}
							/>
							<FilaInterruptor
								etiqueta={t("Contadores al entrar (lealtad, sagas)")}
								valor={opciones.ayudas.contadoresAlEntrar}
								onChange={(v) => ayuda("contadoresAlEntrar", v)}
							/>
							<Fila etiqueta={t("Sparring (rival fantasma)")}>
								<Selector<Intensidad>
									valor={sparring}
									onChange={(v) => useUi.getState().setMesa({ sparring: v })}
									opciones={INTENSIDADES.map((i) => ({ valor: i.valor, etiqueta: t(i.nombre) }))}
								/>
							</Fila>
							<Button
								size="sm"
								variant="outline"
								disabled={!elegido}
								onClick={() => setTapete(true)}
							>
								<Image className="size-3.5" />
								{t("Tapete de este deck…")}
							</Button>
						</section>
						<div className="mt-auto flex flex-col gap-2.5">
							{trabajo && (
								<div className="flex flex-col gap-1">
									<span className="text-xs text-muted-foreground">{trabajo.fase}</span>
									<Progress value={trabajo.total ? (trabajo.hechas / trabajo.total) * 100 : 0} />
								</div>
							)}
							<Button
								size="lg"
								disabled={!elegido || !!trabajo || sinLista}
								onClick={pedirSolitario}
								data-tutorial="mesa-solitario"
							>
								{trabajo ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<Play className="size-4" />
								)}
								{t("Solitario")}
							</Button>
							<Button
								size="lg"
								variant={enSalaServidor ? "default" : "outline"}
								onClick={() => setAmigos(true)}
								data-tutorial="mesa-amigos"
							>
								<Users className="size-4" />
								{enSalaServidor ? t("Volver a la sala") : t("Con amigos…")}
							</Button>
							<Button
								size="lg"
								variant="outline"
								disabled={!!trabajo}
								onClick={() => setLocal(true)}
							>
								<User className="size-4" />
								{t("Partida local…")}
							</Button>
							<Button
								size="lg"
								variant={enSala && !enSalaServidor ? "default" : "outline"}
								onClick={() => setRed(true)}
								data-tutorial="mesa-red"
							>
								<Plug className="size-4" />
								{enSala && !enSalaServidor ? t("Volver a la sala") : t("En red…")}
							</Button>
						</div>
					</aside>
					{elegido && editando && (
						<ListaDialog
							abierto
							onCerrar={() => setEditando(null)}
							deckId={elegido}
							formato={formato}
							lista={editando.lista}
							onGuardada={(l) => {
								setListas((x) =>
									x
										? {
												...x,
												listas: [...x.listas.filter((y) => y.id !== l.id), l].sort((a, b) =>
													a.nombre.localeCompare(b.nombre),
												),
											}
										: x,
								);
								setListaId(l.id);
							}}
							onBorrada={(id) => {
								setListas((x) => (x ? { ...x, listas: x.listas.filter((y) => y.id !== id) } : x));
								setListaId((actual) => (actual === id ? null : actual));
							}}
						/>
					)}
					<SalasDialog
						abierto={amigos}
						onCerrar={() => {
							setAmigos(false);
							useSalas.getState().poner({ enlaceEntrante: null });
						}}
						preparar={preparar}
					/>
					<PartidaLocalDialog
						abierto={local}
						onCerrar={() => setLocal(false)}
						decks={decks}
						deckInicial={elegido}
					/>
					<HistorialGeneralDialog abierto={historial} onCerrar={() => setHistorial(false)} />
					<PropiedadesDialog
						id={propiedades}
						onCerrar={() => setPropiedades(null)}
						onNuevaEtiqueta={(para) => etiquetasUi.editar({ tipo: "nueva", para })}
					/>
					{etiquetasUi.dialogos}
					<RedDialog abierto={red} onCerrar={() => setRed(false)} preparar={preparar} />
					<TapeteDialog abierto={tapete} onCerrar={() => setTapete(false)} deckId={elegido} />
					<Confirmar
						abierto={reemplazar}
						onCerrar={() => setReemplazar(false)}
						titulo={t("¿Empezar otra partida?")}
						descripcion={t("La partida a medias se pierde.")}
						accion={t("Empezar otra")}
						destructiva
						onConfirmar={() => void solitario()}
					/>
					<Confirmar
						abierto={descartar}
						onCerrar={() => setDescartar(false)}
						titulo={t("¿Descartar la partida?")}
						descripcion={t("La partida a medias se pierde.")}
						accion={t("Descartar")}
						destructiva
						onConfirmar={() => {
							void salir().then(() => setEstado({ abierta: false, deckIds: [], guardada: null }));
						}}
					/>
				</div>
			</SeleccionDecks.Provider>
		</AccionesDeck.Provider>
	);
}
