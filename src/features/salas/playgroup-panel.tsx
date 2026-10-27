import {
	Check,
	Copy,
	Crown,
	Eye,
	Link2,
	LogIn,
	LogOut,
	MoreHorizontal,
	Play,
	Shield,
} from "lucide-react";
import { useEffect, useState } from "react";
import type { Formato } from "@/bindings/Formato";
import type { Playgroup } from "@/bindings/Playgroup";
import { CampoNumero, Fila, FilaInterruptor, Selector } from "@/components/campos";
import { Confirmar } from "@/components/confirmar";
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
				{copiado === "codigo" ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
				{copiado === "codigo" ? t("Copiado") : t("Copiar")}
			</Button>
			{enlace && (
				<Button size="sm" variant="outline" onClick={() => copiar(enlace, "enlace")}>
					{copiado === "enlace" ? <Check className="size-3.5" /> : <Link2 className="size-3.5" />}
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
	const [formato, setFormato] = useState<Formato>(preparar.formato);
	const [maximo, setMaximo] = useState(4);
	const [espectadores, setEspectadores] = useState(true);
	const manda = pg.rol === "dueno" || pg.rol === "admin";

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-2.5">
			<div className="flex items-center gap-2.5">
				<h3 className="truncate text-sm font-semibold">{pg.nombre}</h3>
				<Button size="sm" variant="ghost" className="ml-auto" onClick={() => setSalir(true)}>
					<LogOut className="size-3.5" />
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
							onClick={async () => setCodigo(await invitar(pg.id))}
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
									{m.rol === "dueno" && <Crown className="size-3.5 text-amber-500" />}
									{m.rol === "admin" && <Shield className="size-3.5 text-sky-500" />}
									{manda && m.clave !== yo && m.rol !== "dueno" && (
										<DropdownMenu>
											<DropdownMenuTrigger asChild>
												<Button
													size="icon-sm"
													variant="ghost"
													className="ml-auto"
													aria-label={t("Opciones")}
												>
													<MoreHorizontal className="size-3.5" />
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
														onClick={() => void cambiarRol(pg.id, m.clave, "dueno")}
													>
														{t("Pasarle el playgroup")}
													</DropdownMenuItem>
												)}
												<DropdownMenuItem
													className="text-destructive"
													onClick={() => void expulsar(pg.id, m.clave)}
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
												onClick={() => void unirseASala(s.id, "jugador", preparar.nombre)}
											>
												<LogIn className="size-3.5" />
												{t("Entrar")}
											</Button>
										)}
										<Button
											size="sm"
											variant="ghost"
											onClick={() => void unirseASala(s.id, "espectador", preparar.nombre)}
										>
											<Eye className="size-3.5" />
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
							onClick={() =>
								void crearSala(
									{ playgroup: pg.id, formato, maxJugadores: maximo, espectadores },
									preparar.nombre,
								)
							}
						>
							<Play className="size-3.5" />
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
		</div>
	);
}
