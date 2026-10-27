import { Check, Copy, Crown, Eye, Loader2, LogOut, Play, Plug, Users } from "lucide-react";
import { useEffect, useState } from "react";
import type { Formato } from "@/bindings/Formato";
import type { Opciones } from "@/bindings/Opciones";
import type { RolEnSala } from "@/bindings/RolEnSala";
import { Selector } from "@/components/campos";
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
import { COLORES_JUGADOR } from "../colores";
import { crearPartida, empezarRed, refrescarRed, salirDeRed, unirsePartida } from "./conexion-red";

/** Lo que trae la entrada de la mesa: el deck elegido y sus opciones. */
export type Preparacion = {
	deckId: string | null;
	deckNombre: string | null;
	lista: string | null;
	/** El deck pide una lista y no hay ninguna elegida. */
	sinLista: boolean;
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

	const copiar = async () => {
		if (!codigo) return;
		await navigator.clipboard.writeText(codigo);
		setCopiado(true);
		window.setTimeout(() => setCopiado(false), 1500);
	};

	const empezar = () =>
		void empezarRed({
			jugadores: [
				{
					deckId: preparar.deckId ?? "",
					nombre: preparar.nombre.trim() || t("Anfitrión"),
					color: COLORES_JUGADOR[0],
					lista: preparar.lista ?? undefined,
				},
			],
			formato: preparar.formato,
			variantes: { equipos: { tipo: "ninguno" }, archenemy: null, planechase: false },
			opciones: preparar.opciones,
			intensidad: "apagado",
			equipos: [],
		});

	const conDeck = sala.filter((j) => j.rol === "jugador" && j.deck).length;
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
			<div className="flex flex-col gap-1.5">
				<span className="text-xs font-medium">{t("En la sala")}</span>
				<ScrollArea className="max-h-56">
					<ul className="flex flex-col gap-1 p-1">
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
						{sala.length <= 1 && (
							<li className="p-2.5 text-center text-xs text-muted-foreground">
								{t("Todavía no ha llegado nadie.")}
							</li>
						)}
					</ul>
				</ScrollArea>
			</div>
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
					<Button disabled={!preparar.deckId || preparar.sinLista || !!trabajo} onClick={empezar}>
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
