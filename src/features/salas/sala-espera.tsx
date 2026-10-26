import { Crown, Eye, Layers, Loader2, LogOut, Play, Smartphone } from "lucide-react";
import { useState } from "react";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSalas } from "@/stores/salas-store";
import { useTrabajo } from "@/stores/trabajo-store";
import type { Preparacion } from "../mesa/red/red-dialog";
import { empezarSala, invitar, prepararDeck, salirDeSala } from "./conexion-salas";
import { CodigoInvitacion } from "./playgroup-panel";

/**
 * **La sala de espera**: quién hay, con qué deck y cómo llega esta app a
 * cada uno (directo o por relé, y los ms de la malla). Cada jugador prepara
 * su deck; quien creó la sala empieza cuando todos lo tienen. El servidor
 * elige entonces el host y la partida se abre sola.
 */
export function SalaEspera({ preparar }: { preparar: Preparacion }) {
	const { t } = useLang();
	const sala = useSalas((s) => s.sala);
	const yo = useSalas((s) => s.yo);
	const como = useSalas((s) => s.como);
	const deck = useSalas((s) => s.deck);
	const enlaces = useSalas((s) => s.enlaces);
	const trabajo = useTrabajo((s) => (s.actual?.tipo === "mesa" ? s.actual : null));
	const [codigo, setCodigo] = useState<string | null>(null);
	if (!sala) return null;

	const jugadores = sala.miembros.filter((m) => m.como === "jugador");
	const listos = jugadores.filter((m) => m.deck).length;
	const soyCreador = sala.sala.creador === yo;
	const empezada = sala.sala.fase === "juego";
	const puedeEmpezar =
		soyCreador && !empezada && jugadores.length >= 2 && listos === jugadores.length;

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-2.5">
			<div className="flex items-center gap-2.5">
				<h3 className="text-sm font-semibold">{t("Sala de espera")}</h3>
				<span className="text-[11px] text-muted-foreground">
					{t("Jugadores: {cuantos} de {maximo}", {
						cuantos: jugadores.length,
						maximo: sala.sala.maxJugadores,
					})}
				</span>
				<div className="ml-auto">
					{codigo ? (
						<CodigoInvitacion codigo={codigo} />
					) : (
						<Button
							size="sm"
							variant="outline"
							onClick={async () => setCodigo(await invitar(null))}
						>
							{t("Invitar a la sala")}
						</Button>
					)}
				</div>
			</div>
			<ScrollArea className="min-h-0 flex-1">
				<ul className="flex flex-col gap-1 p-1">
					{sala.miembros.map((m) => {
						const e = enlaces[m.clave];
						return (
							<li
								key={m.clave}
								className="flex items-center gap-2.5 rounded-md border bg-card px-2.5 py-1.5 text-xs"
							>
								<span
									className={
										m.enLinea
											? "size-2 shrink-0 rounded-full bg-emerald-500"
											: "size-2 shrink-0 rounded-full bg-muted-foreground/40"
									}
								/>
								<span className="truncate font-medium">
									{m.clave === yo ? t("{nombre} (tú)", { nombre: m.apodo }) : m.apodo}
								</span>
								{m.clave === sala.sala.creador && <Crown className="size-3.5 text-amber-500" />}
								{m.como === "espectador" && <Eye className="size-3.5 text-muted-foreground" />}
								{m.plataforma !== "escritorio" && (
									<Smartphone className="size-3.5 text-muted-foreground" />
								)}
								{sala.host === m.clave && (
									<span className="rounded bg-primary/15 px-1 text-[10px] text-primary">
										{t("Host")}
									</span>
								)}
								<span className="ml-auto flex items-center gap-1 truncate text-[11px] text-muted-foreground">
									{m.como === "jugador" &&
										(m.deck ? (
											<>
												<Layers className="size-3" />
												{m.deck}
											</>
										) : (
											t("Sin deck todavía")
										))}
								</span>
								{m.clave !== yo && (
									<span className="w-24 shrink-0 text-right text-[10px] text-muted-foreground tabular-nums">
										{e
											? e.directo
												? t("{ms} ms · directo", { ms: e.rttMs })
												: t("{ms} ms · por relé", { ms: e.rttMs })
											: "—"}
									</span>
								)}
							</li>
						);
					})}
				</ul>
			</ScrollArea>
			{como === "jugador" && !empezada && (
				<div className="flex items-center gap-2.5 rounded-lg border bg-card/40 p-2.5">
					<Layers className="size-4 text-primary" />
					<span className="min-w-0 flex-1 truncate text-xs">
						{deck
							? t("Juegas con «{deck}».", { deck })
							: preparar.deckNombre
								? t("Elegido en la lista: «{deck}».", { deck: preparar.deckNombre })
								: t("Elige un deck en la lista de la izquierda de «Jugar».")}
					</span>
					<Button
						size="sm"
						variant={deck ? "outline" : "default"}
						disabled={!preparar.deckId || preparar.sinLista || !!trabajo}
						onClick={() =>
							preparar.deckId &&
							void prepararDeck(
								preparar.deckId,
								preparar.lista,
								preparar.formato,
								preparar.deckNombre ?? "",
							)
						}
					>
						{trabajo ? <Loader2 className="size-3.5 animate-spin" /> : null}
						{deck ? t("Cambiar por el elegido") : t("Jugar con este deck")}
					</Button>
				</div>
			)}
			<div className="flex items-center justify-end gap-2.5">
				{empezada ? (
					<p className="mr-auto flex items-center gap-1.5 text-xs text-muted-foreground">
						<Loader2 className="size-3.5 animate-spin" />
						{t("Empezando: el servidor elige quién hace de host…")}
					</p>
				) : (
					!soyCreador && (
						<p className="mr-auto flex items-center gap-1.5 text-xs text-muted-foreground">
							<Loader2 className="size-3.5 animate-spin" />
							{t("Esperando a que quien creó la sala empiece…")}
						</p>
					)
				)}
				<Button variant="outline" onClick={() => void salirDeSala()}>
					<LogOut className="size-4" />
					{t("Salir de la sala")}
				</Button>
				{soyCreador && !empezada && (
					<Button disabled={!puedeEmpezar} onClick={() => void empezarSala()}>
						<Play className="size-4" />
						{t("Empezar")}
					</Button>
				)}
			</div>
		</div>
	);
}
