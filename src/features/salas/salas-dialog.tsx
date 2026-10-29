import { useEffect, useState } from "react";
import type { ComoEnSala } from "@/bindings/ComoEnSala";
import { Selector } from "@/components/campos";
import { Icono } from "@/components/icono";
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
import { cn } from "@/lib/utils";
import { useSalas } from "@/stores/salas-store";
import type { Preparacion } from "../mesa/red/red-dialog";
import { conectar, crearPlaygroup, usarCodigo } from "./conexion-salas";
import { PanelPlaygroup } from "./playgroup-panel";
import { SalaEspera } from "./sala-espera";

/** Lo que se espera al servidor de salas antes de ofrecer reintentar, en ms. */
const ESPERA_SERVIDOR_MS = 10_000;

/**
 * **Jugar con amigos** (Fase F): los playgroups de esta instalación (solo
 * por invitación), unirse con un código y, dentro de una sala, la sala de
 * espera. Sin servidor configurado, lo dice y remite al código directo.
 */
export function SalasDialog({
	abierto,
	onCerrar,
	preparar,
	onConCodigo,
}: {
	abierto: boolean;
	onCerrar: () => void;
	preparar: Preparacion;
	/** Ir a «En red…» (código directo): sin servidor, el camino que queda. */
	onConCodigo: () => void;
}) {
	const { t } = useLang();
	const hayServidor = useSalas((s) => s.hayServidor);
	const conectado = useSalas((s) => s.conectado);
	const playgroups = useSalas((s) => s.playgroups);
	const sala = useSalas((s) => s.sala);
	const [elegido, setElegido] = useState<string | null>(null);
	const [comprobado, setComprobado] = useState(false);
	// Lo que se lleva esperando al servidor: pasado un rato, se ofrece reintentar.
	const [tarda, setTarda] = useState(false);
	const [intento, setIntento] = useState(0);

	useEffect(() => {
		if (!abierto) return;
		setTarda(false);
		void conectar(preparar.nombre).finally(() => setComprobado(true));
		const id = window.setTimeout(() => setTarda(true), ESPERA_SERVIDOR_MS);
		return () => window.clearTimeout(id);
	}, [abierto, preparar.nombre, intento]);
	useEffect(() => {
		if (!elegido || !playgroups.some((p) => p.id === elegido)) {
			setElegido(playgroups[0]?.id ?? null);
		}
	}, [playgroups, elegido]);

	const pg = playgroups.find((p) => p.id === elegido) ?? null;
	return (
		<Dialog open={abierto} onOpenChange={(v) => !v && onCerrar()}>
			<DialogContent className="max-w-4xl">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-1.5">
						<Icono icon="lucide:users" className="size-4 text-primary" />
						{t("Jugar con amigos")}
					</DialogTitle>
					<DialogDescription>
						{t(
							"Tus playgroups, solo por invitación. La partida va directa entre vosotros: el servidor solo os junta y elige quién hace de host.",
						)}
					</DialogDescription>
				</DialogHeader>
				{!comprobado ? (
					<p className="flex items-center gap-1.5 p-2.5 text-xs text-muted-foreground">
						<Icono icon="lucide:loader-2" className="size-3.5 animate-spin" />
						{t("Conectando con el servidor…")}
					</p>
				) : !hayServidor ? (
					<div className="flex flex-col items-start gap-2.5 rounded-md bg-muted/40 p-2.5 text-xs text-muted-foreground">
						<p className="flex items-start gap-1.5">
							<Icono icon="lucide:wifi-off" className="mt-0.5 size-3.5 shrink-0" />
							{t(
								"Esta versión aún no tiene servidor de salas. Puedes jugar en red con un código desde «En red…».",
							)}
						</p>
						<Button
							size="sm"
							onClick={() => {
								onCerrar();
								onConCodigo();
							}}
						>
							<Icono icon="lucide:plug" className="size-4" />
							{t("Jugar con un código…")}
						</Button>
					</div>
				) : !conectado ? (
					<div className="flex flex-col items-start gap-2.5 p-2.5 text-xs text-muted-foreground">
						<p className="flex items-center gap-1.5">
							<Icono icon="lucide:loader-2" className="size-3.5 animate-spin" />
							{t("Conectando con el servidor…")}
						</p>
						{tarda && (
							<div className="flex flex-wrap items-center gap-2.5">
								<span>{t("El servidor no responde.")}</span>
								<Button size="sm" variant="outline" onClick={() => setIntento((n) => n + 1)}>
									<Icono icon="lucide:refresh-cw" className="size-4" />
									{t("Reintentar")}
								</Button>
								<Button
									size="sm"
									variant="ghost"
									onClick={() => {
										onCerrar();
										onConCodigo();
									}}
								>
									{t("Jugar con un código…")}
								</Button>
							</div>
						)}
					</div>
				) : (
					<div className="flex h-[min(560px,70vh)] min-h-0 gap-2.5">
						<aside className="flex w-64 shrink-0 flex-col gap-2.5">
							<ListaPlaygroups elegido={elegido} onElegir={setElegido} />
							<CrearPlaygroup />
							<UnirseConCodigo apodo={preparar.nombre} />
						</aside>
						<section className="flex min-h-0 flex-1 flex-col rounded-lg border bg-card/40 p-2.5">
							{sala ? (
								<SalaEspera preparar={preparar} />
							) : pg ? (
								<PanelPlaygroup pg={pg} preparar={preparar} />
							) : (
								<p className="m-auto max-w-xs text-center text-xs text-muted-foreground">
									{t(
										"Crea un playgroup y pasa su código a tus amigos, o únete al de alguien con el suyo.",
									)}
								</p>
							)}
						</section>
					</div>
				)}
			</DialogContent>
		</Dialog>
	);
}

function ListaPlaygroups({
	elegido,
	onElegir,
}: {
	elegido: string | null;
	onElegir: (id: string) => void;
}) {
	const { t } = useLang();
	const playgroups = useSalas((s) => s.playgroups);
	const sala = useSalas((s) => s.sala);
	return (
		<ScrollArea className="min-h-0 flex-1 rounded-lg border bg-card/40">
			<ul className="flex flex-col gap-1 p-1">
				{playgroups.map((p) => {
					const enLinea = p.miembros.filter((m) => m.enLinea).length;
					return (
						<li key={p.id}>
							<button
								type="button"
								disabled={!!sala}
								onClick={() => onElegir(p.id)}
								className={cn(
									"flex w-full flex-col gap-0.5 rounded-md border bg-card px-2.5 py-1.5 text-left transition hover:border-primary/60 disabled:opacity-60",
									elegido === p.id && !sala && "ring-2 ring-primary",
								)}
							>
								<span className="truncate text-xs font-medium">{p.nombre}</span>
								<span className="text-[10px] text-muted-foreground">
									{p.miembros.length === 1
										? t("1 miembro")
										: t("{n} miembros", { n: p.miembros.length })}{" "}
									· {t("En línea: {cuantos}", { cuantos: enLinea })}
									{p.salas.length > 0 && ` · ${t("Salas: {cuantas}", { cuantas: p.salas.length })}`}
								</span>
							</button>
						</li>
					);
				})}
				{playgroups.length === 0 && (
					<li className="p-2.5 text-center text-[11px] text-muted-foreground">
						{t("Aún no estás en ningún playgroup.")}
					</li>
				)}
			</ul>
		</ScrollArea>
	);
}

function CrearPlaygroup() {
	const { t } = useLang();
	const [nombre, setNombre] = useState("");
	const [ocupado, setOcupado] = useState(false);
	const crear = async () => {
		setOcupado(true);
		if (await crearPlaygroup(nombre.trim())) setNombre("");
		setOcupado(false);
	};
	return (
		<div className="flex gap-1.5">
			<Input
				value={nombre}
				onChange={(e) => setNombre(e.target.value)}
				placeholder={t("Nombre del playgroup")}
				maxLength={48}
				className="h-7 text-xs"
				onKeyDown={(e) => e.key === "Enter" && nombre.trim() && void crear()}
			/>
			<Button size="sm" variant="outline" disabled={!nombre.trim() || ocupado} onClick={crear}>
				<Icono icon="lucide:plus" className="size-3.5" />
				{t("Crear")}
			</Button>
		</div>
	);
}

function UnirseConCodigo({ apodo }: { apodo: string }) {
	const { t } = useLang();
	const [codigo, setCodigo] = useState("");
	const [como, setComo] = useState<ComoEnSala>("jugador");
	const [ocupado, setOcupado] = useState(false);
	const sala = useSalas((s) => s.sala);
	const entrante = useSalas((s) => s.enlaceEntrante);
	// El código de un enlace de invitación entra aquí; unirse lo decide la persona.
	useEffect(() => {
		if (!entrante) return;
		setCodigo(`${entrante.slice(0, 5)}-${entrante.slice(5)}`);
		useSalas.getState().poner({ enlaceEntrante: null });
	}, [entrante]);
	const usar = async () => {
		setOcupado(true);
		if (await usarCodigo(codigo.trim(), como, apodo)) setCodigo("");
		setOcupado(false);
	};
	return (
		<div className="flex flex-col gap-1.5 rounded-lg border bg-card/40 p-2.5">
			<span className="text-xs font-medium">{t("Unirse con un código")}</span>
			<Input
				value={codigo}
				onChange={(e) => setCodigo(e.target.value)}
				placeholder="XXXXX-XXXXX"
				className="h-7 font-mono text-xs uppercase"
				spellCheck={false}
			/>
			<div className="flex items-center gap-1.5">
				<Selector<ComoEnSala>
					valor={como}
					onChange={setComo}
					opciones={[
						{ valor: "jugador", etiqueta: t("Jugar") },
						{ valor: "espectador", etiqueta: t("Solo mirar") },
					]}
				/>
				<Button
					size="sm"
					className="ml-auto"
					disabled={codigo.replace(/[\s-]/g, "").length !== 10 || ocupado || !!sala}
					onClick={usar}
				>
					<Icono icon="lucide:log-in" className="size-3.5" />
					{t("Unirse")}
				</Button>
			</div>
			<p className="text-[10px] text-muted-foreground">
				{t("Vale el código de un playgroup o el de una sala.")}
			</p>
		</div>
	);
}
