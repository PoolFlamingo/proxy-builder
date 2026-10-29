import { useEffect, useState } from "react";
import type { Formato } from "@/bindings/Formato";
import type { Playgroup } from "@/bindings/Playgroup";
import { CampoNumero, Fila, FilaInterruptor, Selector } from "@/components/campos";
import { Confirmar } from "@/components/confirmar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ScrollArea } from "@/components/ui/scroll-area";
import { call } from "@/lib/ipc";
import { useOcupado } from "@/lib/use-ocupado";
import { cn } from "@/lib/utils";
import { useSalas } from "@/stores/salas-store";
import type { Preparacion } from "../mesa/red/red-dialog";
import {
	cambiarRol,
	crearSala,
	expulsar,
	invitar,
	salirDePlaygroup,
	unirseASala,
} from "./conexion-salas";

/** Un código de invitación con su botón de copiar. */
export function CodigoInvitacion({ codigo }: { codigo: string }) {
	const { t } = useLang();
	const [copiado, setCopiado] = useState<"codigo" | "enlace" | null>(null);
	const [enlace, setEnlace] = useState<string | null>(null);
	const bonito = `${codigo.slice(0, 5)}-${codigo.slice(5)}`;
	// El enlace lo escribe Rust (su formato vive en `enlace.rs`); se pide al
	// mostrar el código para que copiarlo sea un clic sin esperas.
	useEffect(() => {
		let vivo = true;
		setEnlace(null);
		call("enlace_invitacion", { codigo })
			.then((e) => vivo && setEnlace(e))
			.catch(() => {});
		return () => {
			vivo = false;
		};
	}, [codigo]);
	const copiar = (texto: string, que: "codigo" | "enlace") => {
		void navigator.clipboard.writeText(texto);
		setCopiado(que);
		window.setTimeout(() => setCopiado(null), 1500);
	};
	return (
		<div className="flex items-center gap-1.5">
			<code className="rounded-md border bg-muted/40 px-2 py-1 font-mono text-sm tracking-widest select-text">
				{bonito}
			</code>
			<Button size="sm" variant="outline" onClick={() => copiar(bonito, "codigo")}>
				{copiado === "codigo" ? (
					<Icono icon="lucide:check" className="size-3.5" />
				) : (
					<Icono icon="lucide:copy" className="size-3.5" />
				)}
				{copiado === "codigo" ? t("Copiado") : t("Copiar")}
			</Button>
			{enlace && (
				<Button size="sm" variant="outline" onClick={() => copiar(enlace, "enlace")}>
					{copiado === "enlace" ? (
						<Icono icon="lucide:check" className="size-3.5" />
					) : (
						<Icono icon="lucide:link-2" className="size-3.5" />
					)}
					{copiado === "enlace" ? t("Copiado") : t("Copiar enlace")}
				</Button>
			)}
		</div>
	);
}

/**
 * **Un playgroup**: sus miembros (con su punto de en línea y su papel), sus
 * salas en espera, invitar, crear sala y, para el dueño y los admins,
 * cambiar papeles o expulsar.
 */
export function PanelPlaygroup({ pg, preparar }: { pg: Playgroup; preparar: Preparacion }) {
	const { t } = useLang();
	const yo = useSalas((s) => s.yo);
	const [codigo, setCodigo] = useState<string | null>(null);
	const [salir, setSalir] = useState(false);
	const [ocupado, envolver] = useOcupado();
	// Lo que no tiene vuelta atrás, con quién: se pregunta antes.
	const [pendiente, setPendiente] = useState<{
		que: "pasar" | "expulsar";
		clave: string;
		apodo: string;
	} | null>(null);
	const [formato, setFormato] = useState<Formato>(preparar.formato);
	const [maximo, setMaximo] = useState(4);
	const [espectadores, setEspectadores] = useState(true);
	const manda = pg.rol === "dueno" || pg.rol === "admin";

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-2.5">
			<div className="flex items-center gap-2.5">
				<h3 className="truncate text-sm font-semibold">{pg.nombre}</h3>
				<Button size="sm" variant="ghost" className="ml-auto" onClick={() => setSalir(true)}>
					<Icono icon="lucide:log-out" className="size-3.5" />
					{t("Salir del playgroup")}
				</Button>
			</div>
			{manda && (
				<div className="flex items-center gap-2.5">
					{codigo ? (
						<CodigoInvitacion codigo={codigo} />
					) : (
						<Button
							size="sm"
							variant="outline"
							disabled={ocupado}
							onClick={() => void envolver(async () => setCodigo(await invitar(pg.id)))}
						>
							{t("Invitar")}
						</Button>
					)}
					<span className="text-[11px] text-muted-foreground">
						{t("El código vale 7 días para quien lo tenga.")}
					</span>
				</div>
			)}
			<div className="grid min-h-0 flex-1 grid-cols-2 gap-2.5">
				<div className="flex min-h-0 flex-col gap-1.5">
					<span className="text-xs font-medium">{t("Miembros")}</span>
					<ScrollArea className="min-h-0 flex-1">
						<ul className="flex flex-col gap-1 p-1">
							{pg.miembros.map((m) => (
								<li
									key={m.clave}
									className="flex items-center gap-1.5 rounded-md border bg-card px-2.5 py-1.5 text-xs"
								>
									<span
										className={
											m.enLinea
												? "size-2 shrink-0 rounded-full bg-emerald-500"
												: "size-2 shrink-0 rounded-full bg-muted-foreground/40"
										}
									/>
									<span className="truncate">
										{m.clave === yo ? t("{nombre} (tú)", { nombre: m.apodo }) : m.apodo}
									</span>
									{m.rol === "dueno" && (
										<Icono icon="game-icons:crown" className="size-3.5 text-amber-500" />
									)}
									{m.rol === "admin" && (
										<Icono icon="game-icons:shield" className="size-3.5 text-sky-500" />
									)}
									{manda && m.clave !== yo && m.rol !== "dueno" && (
										<DropdownMenu>
											<DropdownMenuTrigger asChild>
												<Button
													size="icon-sm"
													variant="ghost"
													className="ml-auto"
													aria-label={t("Opciones")}
												>
													<Icono icon="lucide:more-horizontal" className="size-3.5" />
												</Button>
											</DropdownMenuTrigger>
											<DropdownMenuContent align="end">
												{m.rol === "miembro" ? (
													<DropdownMenuItem
														onClick={() => void cambiarRol(pg.id, m.clave, "admin")}
													>
														{t("Hacer admin")}
													</DropdownMenuItem>
												) : (
													<DropdownMenuItem
														onClick={() => void cambiarRol(pg.id, m.clave, "miembro")}
													>
														{t("Quitar de admin")}
													</DropdownMenuItem>
												)}
												{pg.rol === "dueno" && (
													<DropdownMenuItem
														onClick={() =>
															setPendiente({ que: "pasar", clave: m.clave, apodo: m.apodo })
														}
													>
														{t("Pasarle el playgroup")}
													</DropdownMenuItem>
												)}
												<DropdownMenuItem
													className="text-destructive"
													onClick={() =>
														setPendiente({ que: "expulsar", clave: m.clave, apodo: m.apodo })
													}
												>
													{t("Expulsar")}
												</DropdownMenuItem>
											</DropdownMenuContent>
										</DropdownMenu>
									)}
								</li>
							))}
						</ul>
					</ScrollArea>
				</div>
				<div className="flex min-h-0 flex-col gap-1.5">
					<span className="text-xs font-medium">{t("Salas")}</span>
					<ScrollArea className="min-h-0 flex-1">
						<ul className="flex flex-col gap-1 p-1">
							{pg.salas.map((s) => (
								<li
									key={s.id}
									className="flex flex-col gap-1 rounded-md border bg-card px-2.5 py-1.5 text-xs"
								>
									<div className="flex items-center gap-1.5">
										<span className="truncate font-medium">
											{pg.miembros.find((m) => m.clave === s.creador)?.apodo ?? "?"}
										</span>
										<span className="text-[10px] text-muted-foreground tabular-nums">
											{s.jugadores}/{s.maxJugadores}
										</span>
										<span className="ml-auto text-[10px] text-muted-foreground">
											{s.fase === "espera" ? t("en espera") : t("jugando")}
										</span>
									</div>
									<div className="flex justify-end gap-1">
										{s.fase === "espera" && (
											<Button
												size="sm"
												variant="outline"
												disabled={ocupado}
												onClick={() =>
													void envolver(() => unirseASala(s.id, "jugador", preparar.nombre))
												}
											>
												<Icono icon="lucide:log-in" className="size-3.5" />
												{t("Entrar")}
											</Button>
										)}
										<Button
											size="sm"
											variant="ghost"
											disabled={ocupado}
											onClick={() =>
												void envolver(() => unirseASala(s.id, "espectador", preparar.nombre))
											}
										>
											<Icono icon="lucide:eye" className="size-3.5" />
											{t("Mirar")}
										</Button>
									</div>
								</li>
							))}
							{pg.salas.length === 0 && (
								<li className="p-2.5 text-center text-[11px] text-muted-foreground">
									{t("No hay ninguna sala abierta.")}
								</li>
							)}
						</ul>
					</ScrollArea>
					<div className="flex flex-col gap-1.5 rounded-lg border bg-card/40 p-2.5">
						<span className="text-xs font-medium">{t("Nueva sala")}</span>
						<Fila etiqueta={t("Formato")}>
							<Selector<Formato>
								valor={formato}
								onChange={setFormato}
								opciones={[
									{ valor: "commander", etiqueta: "Commander" },
									{ valor: "constructed", etiqueta: t("60 cartas") },
									{ valor: "limited", etiqueta: t("Limitado") },
									{ valor: "libre", etiqueta: t("Libre") },
								]}
							/>
						</Fila>
						<Fila etiqueta={t("Jugadores")}>
							<CampoNumero
								valor={maximo}
								onChange={(v) => setMaximo(Math.max(2, Math.min(10, v ?? 4)))}
								min={2}
								max={10}
								paso={1}
							/>
						</Fila>
						<FilaInterruptor
							etiqueta={t("Se puede mirar")}
							valor={espectadores}
							onChange={setEspectadores}
						/>
						<Button
							size="sm"
							disabled={ocupado}
							onClick={() =>
								void envolver(() =>
									crearSala(
										{ playgroup: pg.id, formato, maxJugadores: maximo, espectadores },
										preparar.nombre,
									),
								)
							}
						>
							<Icono
								icon={ocupado ? "lucide:loader-2" : "lucide:play"}
								className={cn("size-3.5", ocupado && "animate-spin")}
							/>
							{t("Crear sala")}
						</Button>
					</div>
				</div>
			</div>
			<Confirmar
				abierto={salir}
				onCerrar={() => setSalir(false)}
				titulo={t("¿Salir del playgroup?")}
				descripcion={
					pg.rol === "dueno" && pg.miembros.length > 1
						? t("Antes pásale el playgroup a otro miembro.")
						: t("Para volver hará falta otra invitación.")
				}
				accion={t("Salir")}
				destructiva
				onConfirmar={() => void salirDePlaygroup(pg.id)}
			/>
			<Confirmar
				abierto={pendiente !== null}
				onCerrar={() => setPendiente(null)}
				titulo={
					pendiente?.que === "pasar"
						? t("¿Pasarle el playgroup a {nombre}?", { nombre: pendiente.apodo })
						: t("¿Expulsar a {nombre}?", { nombre: pendiente?.apodo ?? "" })
				}
				descripcion={
					pendiente?.que === "pasar"
						? t("Dejarás de ser quien manda en el playgroup y no podrás deshacerlo.")
						: t("Para volver hará falta otra invitación.")
				}
				accion={pendiente?.que === "pasar" ? t("Pasárselo") : t("Expulsar")}
				destructiva
				onConfirmar={() => {
					if (!pendiente) return;
					if (pendiente.que === "pasar") void cambiarRol(pg.id, pendiente.clave, "dueno");
					else void expulsar(pg.id, pendiente.clave);
				}}
			/>
		</div>
	);
}
