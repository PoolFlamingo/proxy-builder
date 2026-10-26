import {
	Image,
	ListChecks,
	Loader2,
	Pencil,
	Play,
	Plug,
	Plus,
	Search,
	Swords,
	Trash2,
	User,
	Users,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { DeckSummary } from "@/bindings/DeckSummary";
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
import { ScrollArea } from "@/components/ui/scroll-area";
import { PORTADA } from "@/features/library/vista-cuadricula";
import { urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useRed } from "@/stores/red-store";
import { useSalas } from "@/stores/salas-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";
import { SalasDialog } from "../salas/salas-dialog";
import { COLORES_JUGADOR } from "./colores";
import { empezar, retomar, salir } from "./conexion";
import { ListaDialog } from "./listas/lista-dialog";
import { RedDialog } from "./red/red-dialog";
import { INTENSIDADES } from "./sparring/textos";
import { TapeteDialog } from "./tapete-dialog";

/** Los valores de fábrica de un formato (lo mismo que `Opciones::de_fabrica` de Rust). */
export function opcionesDeFabrica(formato: Formato, jugadores: number): Opciones {
	return {
		vidaInicial: formato === "commander" ? 40 : 20,
		manoInicial: 7,
		mulligan: formato === "commander" ? "londresPrimeroGratis" : "londres",
		primeroNoRoba: jugadores <= 2,
		ayudas: {
			robarAlEmpezar: true,
			enderezarAlEmpezar: true,
			impuestoAutomatico: true,
			contadoresAlEntrar: true,
			avisarVidaCero: true,
		},
		limiteVeneno: 10,
		limiteComandante: 21,
		todosTocanTodo: false,
	};
}

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
	const { t, mensaje } = useLang();
	const [decks, setDecks] = useState<DeckSummary[]>([]);
	const [estado, setEstado] = useState<EstadoMesa | null>(null);
	const [filtro, setFiltro] = useState("");
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
	const enSalaServidor = useSalas((s) => s.sala !== null);
	const enSala = useRed((s) => s.papel !== null);
	const trabajo = useTrabajo((s) => (s.actual?.tipo === "mesa" ? s.actual : null));

	useEffect(() => {
		call("deck_list")
			.then((d) => {
				setDecks(d);
				const reciente = [...d].sort((a, b) => (b.openedAt ?? 0) - (a.openedAt ?? 0))[0];
				if (reciente) setElegido((e) => e ?? reciente.id);
			})
			.catch((e) => toast.error(mensaje(e)));
		call("mesa_estado")
			.then(setEstado)
			.catch(() => setEstado(null));
	}, [mensaje]);

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

	const visibles = useMemo(() => {
		const f = filtro.trim().toLowerCase();
		return decks.filter((d) => !f || d.name.toLowerCase().includes(f));
	}, [decks, filtro]);

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
			variantes: { equipos: { tipo: "ninguno" }, archenemy: null, planechase: false },
			opciones,
			intensidad: sparring,
		});
	};

	const cambiarFormato = (f: Formato) => {
		setFormato(f);
		setOpciones(opcionesDeFabrica(f, 1));
	};
	const ayuda = (k: keyof Opciones["ayudas"], v: boolean) =>
		setOpciones({ ...opciones, ayudas: { ...opciones.ayudas, [k]: v } });

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
		<div className="flex h-full min-h-0 gap-2.5 p-2.5" data-tutorial="mesa-entrada">
			<section className="flex min-h-0 flex-1 flex-col gap-2.5 rounded-lg border bg-card/40 p-2.5">
				<div className="flex items-center gap-2.5">
					<Swords className="size-4 text-primary" />
					<h2 className="text-sm font-semibold">{t("Elige un deck")}</h2>
					<div className="relative ml-auto w-64">
						<Search className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
						<Input
							value={filtro}
							onChange={(e) => setFiltro(e.target.value)}
							placeholder={t("Buscar deck…")}
							className="h-7 pl-7 text-xs"
						/>
					</div>
				</div>
				<ScrollArea className="min-h-0 flex-1">
					<div className="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-2.5 p-1">
						{visibles.map((d) => (
							<button
								key={d.id}
								type="button"
								onClick={() => setElegido(d.id)}
								onDoubleClick={() => {
									setElegido(d.id);
									pedirSolitario();
								}}
								className={cn(
									"flex flex-col gap-1.5 rounded-lg border bg-card p-1.5 text-left transition hover:border-primary/60",
									elegido === d.id && "ring-2 ring-primary",
								)}
							>
								<div className="aspect-[63/88] overflow-hidden rounded-md bg-muted">
									{d.thumbnail && (
										<img
											src={urlMiniatura(d.thumbnail, PORTADA, 244)}
											alt=""
											loading="lazy"
											className="size-full object-cover"
										/>
									)}
								</div>
								<span className="line-clamp-2 text-xs font-medium">{d.name}</span>
								<span className="text-[10px] text-muted-foreground">
									{t("{n} cartas", { n: d.cardCount })}
								</span>
							</button>
						))}
						{visibles.length === 0 && (
							<p className="col-span-full p-6 text-center text-xs text-muted-foreground">
								{decks.length === 0
									? t("No hay decks en la biblioteca. Monta uno en la pestaña Deck.")
									: t("Ningún deck coincide con la búsqueda.")}
							</p>
						)}
					</div>
				</ScrollArea>
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
								<Button size="sm" variant="outline" onClick={() => setEditando({ lista: null })}>
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
					<Button size="sm" variant="outline" disabled={!elegido} onClick={() => setTapete(true)}>
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
						{trabajo ? <Loader2 className="size-4 animate-spin" /> : <Play className="size-4" />}
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
			<SalasDialog abierto={amigos} onCerrar={() => setAmigos(false)} preparar={preparar} />
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
	);
}
