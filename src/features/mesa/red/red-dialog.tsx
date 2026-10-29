import { DndContext, type DragEndEvent, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { Check, Copy, Crown, Eye, Loader2, LogOut, Play, Plug, Shuffle, Users } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { Formato } from "@/bindings/Formato";
import type { ListasDeDeck } from "@/bindings/ListasDeDeck";
import type { Opciones } from "@/bindings/Opciones";
import type { RolEnSala } from "@/bindings/RolEnSala";
import { FilaInterruptor, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useRed } from "@/stores/red-store";
import { useTrabajo } from "@/stores/trabajo-store";
import {
	AjustesDePartida,
	AvisoDeEquipos,
	FilaEmpieza,
	FilaOrdenable,
	SelectorDeEquipo,
} from "../ajustes-partida";
import { COLORES_JUGADOR } from "../colores";
import {
	AJUSTES_LOCALES,
	type AjustesLocal,
	type AsientoLocal,
	ajustesTrasOrden,
	ajustesTrasQuitar,
	aplicarOrden,
	construirConfig,
	ordenAlAzar,
	ordenTrasMover,
	problemaDeEquipos,
} from "../config-local";
import { ID_ANFITRION, mismoOrden, sentados, sincronizarAsientos } from "../config-sala";
import {
	cambiarApodo,
	cambiarDeckAnfitrion,
	cambiarDeckInvitado,
	crearPartida,
	empezarRed,
	refrescarRed,
	salirDeRed,
	unirsePartida,
} from "./conexion-red";
import { type EleccionAplicada, EleccionEnSala } from "./eleccion-en-sala";

/** Lo que trae la entrada de la mesa: el deck elegido y sus opciones. */
export type Preparacion = {
	deckId: string | null;
	deckNombre: string | null;
	lista: string | null;
	/** El deck pide una lista y no hay ninguna elegida. */
	sinLista: boolean;
	/** El deck pasa del tamaño del formato: no vale jugar con todo el deck. */
	pideLista: boolean;
	/** Las listas para jugar del deck elegido (`null` mientras se leen). */
	listas: ListasDeDeck | null;
	/** Ya se leyeron las listas del deck elegido (o no hay deck). */
	listo: boolean;
	/** Cambia el deck y la lista desde la sala de espera (son los de la entrada de «Jugar»). */
	elegirDeck: (id: string) => void;
	elegirLista: (id: string | null) => void;
	formato: Formato;
	opciones: Opciones;
	nombre: string;
};

/**
 * **Jugar en red con un código** (sin servidor): crear una partida y pasar
 * el código, o pegar el de otro para jugar o mirar. Con la partida creada,
 * la sala de espera: quién hay, con qué deck, y «Empezar» para el
 * anfitrión. Las salas con servidor llegan con la Fase F.
 */
export function RedDialog({
	abierto,
	onCerrar,
	preparar,
}: {
	abierto: boolean;
	onCerrar: () => void;
	preparar: Preparacion;
}) {
	const { t } = useLang();
	const papel = useRed((s) => s.papel);
	useEffect(() => {
		if (abierto) void refrescarRed();
	}, [abierto]);

	return (
		<Dialog open={abierto} onOpenChange={(v) => !v && onCerrar()}>
			<DialogContent className="max-w-xl">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-1.5">
						<Plug className="size-4 text-primary" />
						{t("Jugar en red")}
					</DialogTitle>
					<DialogDescription>
						{t(
							"Sin servidor: quien crea la partida hace de anfitrión y pasa un código a los demás.",
						)}
					</DialogDescription>
				</DialogHeader>
				{papel === null ? <Entrar preparar={preparar} /> : <SalaDeEspera preparar={preparar} />}
			</DialogContent>
		</Dialog>
	);
}

function Entrar({ preparar }: { preparar: Preparacion }) {
	const { t } = useLang();
	const [codigo, setCodigo] = useState("");
	const [rol, setRol] = useState<RolEnSala>("jugador");
	const trabajo = useTrabajo((s) => (s.actual?.tipo === "mesa" ? s.actual : null));
	const [ocupado, setOcupado] = useState(false);
	const sinDeck = !preparar.deckId || preparar.sinLista;

	const crear = async () => {
		setOcupado(true);
		await crearPartida(preparar.nombre, preparar.deckNombre);
		setOcupado(false);
	};
	const unirse = async () => {
		setOcupado(true);
		await unirsePartida({
			codigo: codigo.trim(),
			apodo: preparar.nombre,
			rol,
			deckId: rol === "jugador" ? preparar.deckId : null,
			lista: rol === "jugador" ? preparar.lista : null,
			formato: preparar.formato,
		});
		setOcupado(false);
	};

	return (
		<div className="grid grid-cols-2 gap-2.5">
			<section className="flex flex-col gap-2.5 rounded-lg border bg-card/40 p-2.5">
				<h3 className="flex items-center gap-1.5 text-xs font-semibold">
					<Crown className="size-3.5 text-primary" />
					{t("Crear una partida")}
				</h3>
				<p className="text-xs text-muted-foreground">
					{preparar.deckNombre
						? t("Juegas con «{deck}». Pasa el código a los demás.", { deck: preparar.deckNombre })
						: t("Elige antes un deck en la lista.")}
				</p>
				<Button className="mt-auto" disabled={sinDeck || ocupado || !!trabajo} onClick={crear}>
					{ocupado && !trabajo ? (
						<Loader2 className="size-4 animate-spin" />
					) : (
						<Play className="size-4" />
					)}
					{t("Crear partida")}
				</Button>
			</section>
			<section className="flex flex-col gap-2.5 rounded-lg border bg-card/40 p-2.5">
				<h3 className="flex items-center gap-1.5 text-xs font-semibold">
					<Users className="size-3.5 text-primary" />
					{t("Unirse con un código")}
				</h3>
				<Input
					value={codigo}
					onChange={(e) => setCodigo(e.target.value)}
					placeholder="PB1-…"
					className="h-7 font-mono text-xs"
					spellCheck={false}
				/>
				<Selector<RolEnSala>
					valor={rol}
					onChange={setRol}
					opciones={[
						{ valor: "jugador", etiqueta: t("Jugar") },
						{ valor: "espectador", etiqueta: t("Solo mirar") },
					]}
				/>
				<Button
					className="mt-auto"
					disabled={
						!codigo.trim().startsWith("PB1-") ||
						(rol === "jugador" && sinDeck) ||
						ocupado ||
						!!trabajo
					}
					onClick={unirse}
				>
					{trabajo ? <Loader2 className="size-4 animate-spin" /> : <Plug className="size-4" />}
					{t("Unirse")}
				</Button>
			</section>
		</div>
	);
}

function SalaDeEspera({ preparar }: { preparar: Preparacion }) {
	const { t } = useLang();
	const papel = useRed((s) => s.papel);
	const codigo = useRed((s) => s.codigo);
	const sala = useRed((s) => s.sala);
	const trabajo = useTrabajo((s) => (s.actual?.tipo === "mesa" ? s.actual : null));
	const [copiado, setCopiado] = useState(false);
	const anfitrion = papel === "anfitrion";
	// El anfitrión solo dice con qué deck se le ve (lo prepara al empezar); un
	// invitado lo prepara y se lo manda al anfitrión.
	const aplicarDeck = useCallback(
		(e: EleccionAplicada) => {
			if (anfitrion) void cambiarDeckAnfitrion(e.deckId);
			else void cambiarDeckInvitado(e.deckId, e.lista, e.formato);
		},
		[anfitrion],
	);

	// Lo que decide el anfitrión: asientos (él, el primero), equipos y ajustes.
	const [ajustes, setAjustes] = useState<AjustesLocal>({
		...AJUSTES_LOCALES,
		formato: preparar.formato,
	});
	// En red el campo de los demás va cerrado de fábrica (ver `Opciones`).
	const [campoProtegido, setCampoProtegido] = useState(true);
	const [ids, setIds] = useState<number[]>([ID_ANFITRION]);
	const idsActuales = useRef(ids);
	const [equipos, setEquipos] = useState<Record<number, number | null>>({});
	const sensores = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

	// Quien llega se sienta al final; quien se va, se levanta.
	useEffect(() => {
		const r = sincronizarAsientos(idsActuales.current, sentados(sala));
		if (mismoOrden(idsActuales.current, r.ids)) return;
		idsActuales.current = r.ids;
		setIds(r.ids);
		if (r.quitados.length) {
			setAjustes((a) => r.quitados.reduce((x, i) => ajustesTrasQuitar(x, i), a));
		}
	}, [sala]);

	const reordenar = (orden: number[]) => {
		const nuevo = aplicarOrden(idsActuales.current, orden);
		idsActuales.current = nuevo;
		setIds(nuevo);
		setAjustes((a) => ajustesTrasOrden(a, orden));
	};
	const alSoltar = (e: DragEndEvent) => {
		const de = Number(e.active.id);
		const a = e.over ? Number(e.over.id) : de;
		// El anfitrión se queda en el primer asiento.
		if (de === 0 || a === 0 || de === a) return;
		reordenar(ordenTrasMover(ids.length, de, a));
	};

	const copiar = async () => {
		if (!codigo) return;
		await navigator.clipboard.writeText(codigo);
		setCopiado(true);
		window.setTimeout(() => setCopiado(false), 1500);
	};

	const nombreDe = (id: number) =>
		id === ID_ANFITRION
			? preparar.nombre.trim() || t("Anfitrión")
			: sala.find((j) => j.id === id)?.apodo || t("Jugador");
	const asientos: AsientoLocal[] = ids.map((id) => ({
		nombre: nombreDe(id),
		deckId: id === ID_ANFITRION ? (preparar.deckId ?? "") : "",
		equipo: equipos[id] ?? null,
		lista: id === ID_ANFITRION ? (preparar.lista ?? undefined) : undefined,
	}));
	const nombreDeAsiento = (i: number) => asientos[i]?.nombre || `${t("Jugador")} ${i + 1}`;
	const problema = problemaDeEquipos(asientos, ajustes.modo);
	const porEquipos = ajustes.modo === "equipos";

	const empezar = () => {
		const config = construirConfig(
			asientos,
			ajustes,
			(n) => t("Equipo {n}", { n }),
			t("Contra el archienemigo"),
		);
		// Sin equipos valen las opciones que se eligieron al preparar la mesa.
		const base = ajustes.modo === "ninguno" ? preparar.opciones : config.opciones;
		if (base) config.opciones = { ...base, campoAjenoProtegido: campoProtegido };
		void empezarRed(config, ids.slice(1));
	};

	const conDeck = sala.filter((j) => j.rol === "jugador" && j.deck).length;
	const sentadosAhora = new Set(ids);
	const aparte = sala.filter((j) => !sentadosAhora.has(j.id));
	return (
		<div className="flex flex-col gap-2.5">
			{anfitrion && codigo && (
				<div className="flex flex-col gap-1.5">
					<span className="text-xs font-medium">{t("Código de la partida")}</span>
					<div className="flex items-start gap-1.5">
						<code className="max-h-24 flex-1 overflow-hidden rounded-md border bg-muted/40 p-1.5 font-mono text-[10px] break-all select-text">
							{codigo}
						</code>
						<Button size="sm" variant="outline" onClick={() => void copiar()}>
							{copiado ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
							{copiado ? t("Copiado") : t("Copiar")}
						</Button>
					</div>
				</div>
			)}
			<ScrollArea className="max-h-[55vh]">
				<div className="flex flex-col gap-2.5 p-1">
					<EleccionEnSala
						preparar={preparar}
						juega={papel !== "espectador"}
						alNombre={cambiarApodo}
						aplicar={aplicarDeck}
					/>
					<div className="flex flex-col gap-1.5">
						<span className="text-xs font-medium">{t("En la sala")}</span>
						{anfitrion ? (
							<DndContext sensors={sensores} onDragEnd={alSoltar}>
								<ul className="flex flex-col gap-1">
									{ids.map((id, i) => {
										const fila = sala.find((j) => j.id === id);
										const detalle =
											id === ID_ANFITRION ? (preparar.deckNombre ?? "") : (fila?.deck ?? "");
										const contenido = (
											<>
												<span
													className="size-2.5 shrink-0 rounded-full"
													style={{ background: COLORES_JUGADOR[i % COLORES_JUGADOR.length] }}
												/>
												<span className="truncate text-xs font-medium">{nombreDe(id)}</span>
												{id === ID_ANFITRION && <Crown className="size-3.5 text-amber-500" />}
												<span className="ml-auto truncate text-[11px] text-muted-foreground">
													{detalle}
												</span>
												{porEquipos && (
													<SelectorDeEquipo
														equipo={equipos[id] ?? null}
														onChange={(e) => setEquipos((x) => ({ ...x, [id]: e }))}
													/>
												)}
											</>
										);
										return id === ID_ANFITRION ? (
											<li
												key={id}
												className="flex items-center gap-2.5 rounded-md border bg-card px-2.5 py-1.5 pl-9"
											>
												{contenido}
											</li>
										) : (
											<li key={id}>
												<FilaOrdenable
													i={i}
													etiqueta={t("Mover a {jugador}", { jugador: nombreDe(id) })}
												>
													<div className="flex min-w-0 flex-1 items-center gap-2.5 rounded-md border bg-card px-2.5 py-1.5">
														{contenido}
													</div>
												</FilaOrdenable>
											</li>
										);
									})}
								</ul>
							</DndContext>
						) : (
							<ul className="flex flex-col gap-1">
								{sala.map((j, i) => (
									<FilaSala
										key={`${i}-${j.apodo}`}
										nombre={j.apodo || t("Jugador")}
										detalle={
											j.rol === "espectador" ? t("Mirando") : (j.deck ?? t("Preparando su deck…"))
										}
										color={j.rol === "jugador" ? COLORES_JUGADOR[i % COLORES_JUGADOR.length] : null}
										// El anfitrión va el primero.
										icono={
											i === 0 ? (
												<Crown className="size-3.5 text-amber-500" />
											) : j.rol === "espectador" ? (
												<Eye className="size-3.5" />
											) : null
										}
									/>
								))}
							</ul>
						)}
						{anfitrion && aparte.length > 0 && (
							<ul className="flex flex-col gap-1">
								{aparte.map((j) => (
									<FilaSala
										key={j.id}
										nombre={j.apodo || t("Jugador")}
										detalle={
											j.rol === "espectador" ? t("Mirando") : (j.deck ?? t("Preparando su deck…"))
										}
										color={null}
										icono={j.rol === "espectador" ? <Eye className="size-3.5" /> : null}
									/>
								))}
							</ul>
						)}
						{sala.length <= 1 && (
							<p className="p-2.5 text-center text-xs text-muted-foreground">
								{t("Todavía no ha llegado nadie.")}
							</p>
						)}
					</div>
					{anfitrion && (
						<>
							{ids.length > 2 && (
								<div className="flex items-center gap-2">
									<Button
										size="sm"
										variant="outline"
										onClick={() => reordenar([0, ...ordenAlAzar(ids.length - 1).map((i) => i + 1)])}
									>
										<Shuffle className="size-3.5" />
										{t("Mezclar los asientos")}
									</Button>
									<span className="text-[11px] text-muted-foreground">
										{t("Tú, el anfitrión, te quedas en el primer asiento.")}
									</span>
								</div>
							)}
							<AjustesDePartida
								ajustes={ajustes}
								onChange={(c) => setAjustes((a) => ({ ...a, ...c }))}
								formato={preparar.formato}
								asientos={ids.length}
								nombreDeAsiento={nombreDeAsiento}
							/>
							<FilaEmpieza
								ajustes={ajustes}
								onChange={(c) => setAjustes((a) => ({ ...a, ...c }))}
								asientos={ids.length}
								nombreDeAsiento={nombreDeAsiento}
							/>
							<FilaInterruptor
								etiqueta={t("Proteger el campo de los demás")}
								ayuda={t(
									"Solo tocas lo que controlas en el campo; lo de otro se coge con «Tomar el control». Cementerio, exilio y pila siguen abiertos.",
								)}
								valor={campoProtegido}
								onChange={setCampoProtegido}
							/>
							<AvisoDeEquipos asientos={asientos} modo={ajustes.modo} />
						</>
					)}
				</div>
			</ScrollArea>
			{anfitrion ? (
				<p className="text-[11px] text-muted-foreground">
					{t(
						"Quien no haya mandado su deck al empezar se queda fuera; los que miran, siguen mirando.",
					)}
				</p>
			) : (
				<p className="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Loader2 className="size-3.5 animate-spin" />
					{t("Esperando a que el anfitrión empiece…")}
				</p>
			)}
			<div className="flex justify-end gap-2.5">
				<Button variant="outline" onClick={() => void salirDeRed()}>
					<LogOut className="size-4" />
					{anfitrion ? t("Cerrar la partida") : t("Salir")}
				</Button>
				{anfitrion && (
					<Button
						disabled={!preparar.deckId || preparar.sinLista || !!trabajo || problema !== null}
						onClick={empezar}
					>
						{trabajo ? <Loader2 className="size-4 animate-spin" /> : <Play className="size-4" />}
						{conDeck <= 1 ? t("Empezar (solo tú)") : t("Empezar ({n} jugadores)", { n: conDeck })}
					</Button>
				)}
			</div>
		</div>
	);
}

function FilaSala({
	nombre,
	detalle,
	color,
	icono,
}: {
	nombre: string;
	detalle: string;
	color: string | null;
	icono: React.ReactNode;
}) {
	return (
		<li className="flex items-center gap-2.5 rounded-md border bg-card px-2.5 py-1.5">
			<span
				className="size-2.5 shrink-0 rounded-full"
				style={{ background: color ?? "var(--muted-foreground)" }}
			/>
			<span className="truncate text-xs font-medium">{nombre}</span>
			{icono}
			<span className="ml-auto truncate text-[11px] text-muted-foreground">{detalle}</span>
		</li>
	);
}
