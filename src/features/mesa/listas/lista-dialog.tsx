import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { AvisoLista } from "@/bindings/AvisoLista";
import type { CartaDeLista } from "@/bindings/CartaDeLista";
import type { Formato } from "@/bindings/Formato";
import type { ListaDeJuego } from "@/bindings/ListaDeJuego";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { BotonMenu, FilaMenu, HojaMenu } from "@/components/hoja-menu";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { PORTADA } from "@/features/library/vista-cuadricula";
import { useEstadoParaCerrar } from "@/lib/dialogos-abiertos";
import { useFormato } from "@/lib/formato";
import { urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useMesaMovil } from "../formato-mesa";
import {
	alternarCarta,
	alternarComandante,
	conCopias,
	cuantas,
	type Eleccion,
	eleccionDeFabrica,
	impide,
} from "./eleccion";

type T = (es: string, vars?: Record<string, string | number>) => string;

/** Un aviso de la comprobación, en palabras. */
export function textoAviso(a: AvisoLista, t: T): string {
	switch (a.tipo) {
		case "tamano":
			return a.hasta === null
				? t("Lleva {n} cartas y el formato pide al menos {desde}.", { n: a.hay, desde: a.desde })
				: a.hasta === a.desde
					? t("Lleva {n} cartas y el formato pide {desde}.", { n: a.hay, desde: a.desde })
					: t("Lleva {n} cartas y el formato pide de {desde} a {hasta}.", {
							n: a.hay,
							desde: a.desde,
							hasta: a.hasta,
						});
		case "sinComandante":
			return t("Falta el comandante.");
		case "demasiadosComandantes":
			return t("Como mucho, dos comandantes.");
		case "noPuedeSerComandante":
			return t("{carta} no puede ser comandante.", { carta: a.nombre });
		case "sinPareja":
			return t("Dos comandantes solo con compañero o trasfondo.");
		case "repetida":
			return t(
				"{carta} va {veces} veces. Si son copias que hacen de ficha, márcalas en el deck (clic derecho › Marca › Copia).",
				{ carta: a.nombre, veces: a.veces },
			);
		case "fueraDeIdentidad":
			return t("{carta} se sale de la identidad de color del comandante.", { carta: a.nombre });
		case "yaNoEstan":
			return t("{n} cartas de la lista ya no están en el deck.", { n: a.n });
	}
}

const MARCAS: Record<string, string> = {
	variante: "Variante",
	opcional: "Opcional",
	banquillo: "Banquillo",
};

/**
 * **Editor de una lista para jugar**: de un deck con más cartas de las que
 * se juegan, el comandante (una copia concreta: la variante de diseño que se
 * quiera) y las demás. El deck no cambia. Rust comprueba la lista a cada
 * cambio y los avisos salen abajo; se puede guardar a medias, pero una lista
 * con avisos rojos no deja empezar la partida.
 *
 * En un móvil las cartas son lo primero: «Comandante» y «Cartas» son pestañas
 * a toda la altura, lo demás se queda en filas sueltas (de lado, en una columna
 * a la izquierda) y los avisos en una hoja.
 */
export function ListaDialog({
	abierto,
	onCerrar,
	deckId,
	formato,
	lista,
	onGuardada,
	onBorrada,
}: {
	abierto: boolean;
	onCerrar: () => void;
	deckId: string;
	formato: Formato;
	/** La que se edita, o `null` para una nueva. */
	lista: ListaDeJuego | null;
	onGuardada: (l: ListaDeJuego) => void;
	onBorrada: (id: string) => void;
}) {
	const { t, mensaje } = useLang();
	const movil = useMesaMovil();
	const { horizontal } = useFormato();
	const lado = movil && horizontal;
	const [pestana, setPestana] = useState<"comandante" | "cartas">("cartas");
	const [verAvisos, setVerAvisos] = useState(false);
	const [todas, setTodas] = useState<CartaDeLista[] | null>(null);
	const [nombre, setNombre] = useState("");
	const [eleccion, setEleccion] = useState<Eleccion>({ comandantes: [], cartas: [] });
	const [avisos, setAvisos] = useState<AvisoLista[]>([]);
	const [filtro, setFiltro] = useState("");
	const [soloElegidas, setSoloElegidas] = useState(false);
	const [guardando, setGuardando] = useState(false);
	// Quien abre puede pasar una función nueva en cada pintado: que no relea.
	const cerrar = useRef(onCerrar);
	cerrar.current = onCerrar;
	// Lo que había al abrir, para saber si hay cambios sin guardar.
	const alAbrir = useRef("");
	const commander = formato === "commander";
	const singleton = commander;

	// Las cartas del deck, al abrir.
	useEffect(() => {
		if (!abierto) return;
		let vivo = true;
		setTodas(null);
		call("mesa_lista_cartas", { deck: deckId })
			.then((c) => {
				if (!vivo) return;
				const inicial = lista
					? { comandantes: lista.comandantes, cartas: lista.cartas }
					: eleccionDeFabrica(c);
				alAbrir.current = JSON.stringify([lista?.nombre ?? "", inicial]);
				setTodas(c);
				setNombre(lista?.nombre ?? "");
				setEleccion(inicial);
				setPestana(inicial.comandantes.length === 0 ? "comandante" : "cartas");
			})
			.catch((e) => {
				toast.error(mensaje(e));
				cerrar.current();
			});
		return () => {
			vivo = false;
		};
	}, [abierto, deckId, lista, mensaje]);

	// La comprobación de Rust, un poco después de cada cambio.
	useEffect(() => {
		if (!abierto || !todas) return;
		const id = window.setTimeout(() => {
			call("mesa_lista_comprobar", {
				deck: deckId,
				formato,
				lista: { id: lista?.id ?? "", nombre, ...eleccion },
			})
				.then(setAvisos)
				.catch(() => setAvisos([]));
		}, 250);
		return () => window.clearTimeout(id);
	}, [abierto, todas, deckId, formato, eleccion, lista?.id, nombre]);

	const grupos = useMemo(() => {
		const f = filtro.trim().toLowerCase();
		const elegidas = new Set([...eleccion.comandantes, ...eleccion.cartas]);
		const m = new Map<string, CartaDeLista[]>();
		for (const c of todas ?? []) {
			if (f && !c.nombre.toLowerCase().includes(f) && !(c.tipo ?? "").toLowerCase().includes(f))
				continue;
			if (soloElegidas && !elegidas.has(c.id)) continue;
			m.set(c.nombre, [...(m.get(c.nombre) ?? []), c]);
		}
		return [...m.entries()].sort(([a], [b]) => a.localeCompare(b));
	}, [todas, filtro, soloElegidas, eleccion]);

	const candidatos = useMemo(() => {
		const f = movil ? filtro.trim().toLowerCase() : "";
		return (todas ?? []).filter(
			(c) =>
				c.puedeSerComandante &&
				(!f || c.nombre.toLowerCase().includes(f) || (c.tipo ?? "").toLowerCase().includes(f)),
		);
	}, [todas, filtro, movil]);
	const n = cuantas(eleccion);
	const pide = commander ? 100 : null;
	const bloquea = avisos.some(impide);
	const sinGuardar = todas !== null && JSON.stringify([nombre, eleccion]) !== alAbrir.current;
	useEstadoParaCerrar({ enCurso: guardando, sinGuardar });

	const guardar = async () => {
		setGuardando(true);
		try {
			const l = await call("mesa_lista_guardar", {
				deck: deckId,
				lista: { id: lista?.id ?? "", nombre, ...eleccion },
			});
			onGuardada(l);
			onCerrar();
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setGuardando(false);
		}
	};
	const borrar = async () => {
		if (!lista) return;
		try {
			await call("mesa_lista_borrar", { deck: deckId, id: lista.id });
			onBorrada(lista.id);
			onCerrar();
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	// En el móvil, tres por fila y a más resolución: se tienen que poder reconocer.
	const anchoCarta = movil ? 92 : 75;
	const miniatura = (c: CartaDeLista, elegida: boolean, alPulsar: () => void, corona = false) => (
		<button
			key={c.id}
			type="button"
			onClick={alPulsar}
			className={cn(
				"relative shrink-0 overflow-hidden rounded-[5%] bg-muted transition",
				movil ? "h-[129px] w-[92px]" : "h-[105px] w-[75px]",
				elegida ? "ring-2 ring-primary" : "opacity-45 hover:opacity-80",
			)}
			aria-pressed={elegida}
			aria-label={c.nombre}
		>
			{c.imagen ? (
				<img
					src={urlMiniatura(c.imagen, PORTADA, movil ? 244 : 146)}
					alt=""
					loading="lazy"
					className="size-full object-cover"
					draggable={false}
				/>
			) : (
				<span className="flex size-full items-center p-1 text-center text-[9px]">{c.nombre}</span>
			)}
			{elegida && (
				<span className="absolute top-0.5 right-0.5 rounded-full bg-primary p-0.5 text-primary-foreground">
					{corona ? (
						<Icono icon="game-icons:crown" className="size-2.5" />
					) : (
						<Icono icon="lucide:check" className="size-2.5" />
					)}
				</span>
			)}
			{c.marca && MARCAS[c.marca] && (
				<span className="absolute inset-x-0 bottom-0 bg-violet-600/85 text-center text-[8px] text-white">
					{t(MARCAS[c.marca])}
				</span>
			)}
		</button>
	);

	const rejillaComandantes = (
		<div className="flex flex-wrap gap-1.5 p-1">
			{candidatos.length === 0 && (
				<p className="text-[11px] text-muted-foreground">
					{t("Ninguna carta del deck puede ser comandante.")}
				</p>
			)}
			{candidatos.map((c) =>
				miniatura(
					c,
					eleccion.comandantes.includes(c.id),
					() => setEleccion((e) => alternarComandante(e, c, todas ?? [])),
					true,
				),
			)}
		</div>
	);
	const rejillaCartas = (
		<div className="flex flex-wrap content-start gap-2.5 p-2">
			{grupos.map(([nombreCarta, copias]) => {
				const elegidas = copias.filter(
					(c) => eleccion.cartas.includes(c.id) || eleccion.comandantes.includes(c.id),
				).length;
				const primera = copias[0];
				// Las básicas (sin límite): una casilla y cuántas.
				if (primera?.sinLimite && copias.length > 1)
					return (
						<div key={nombreCarta} className="flex flex-col items-center gap-1">
							{miniatura(primera, elegidas > 0, () =>
								setEleccion((e) => conCopias(e, copias, elegidas > 0 ? 0 : copias.length)),
							)}
							<div className="flex items-center gap-1 text-[11px]">
								<Button
									size="icon"
									variant="outline"
									className="size-5"
									aria-label={t("Una menos")}
									onClick={() => setEleccion((e) => conCopias(e, copias, elegidas - 1))}
								>
									−
								</Button>
								<span className="min-w-10 text-center tabular-nums">
									{elegidas}/{copias.length}
								</span>
								<Button
									size="icon"
									variant="outline"
									className="size-5"
									aria-label={t("Una más")}
									onClick={() => setEleccion((e) => conCopias(e, copias, elegidas + 1))}
								>
									+
								</Button>
							</div>
							<span className="truncate text-center text-[10px]" style={{ width: anchoCarta }}>
								{nombreCarta}
							</span>
						</div>
					);
				return (
					<div key={nombreCarta} className="flex max-w-full flex-col gap-1">
						<div className="flex flex-wrap gap-1">
							{copias.map((c) =>
								miniatura(
									c,
									eleccion.cartas.includes(c.id) || eleccion.comandantes.includes(c.id),
									() => setEleccion((e) => alternarCarta(e, c, todas ?? [], singleton)),
									eleccion.comandantes.includes(c.id),
								),
							)}
						</div>
						<span
							className="truncate text-[10px]"
							style={{ maxWidth: `min(${copias.length * (anchoCarta + 4) - 4}px, 100%)` }}
						>
							{nombreCarta}
						</span>
					</div>
				);
			})}
			{grupos.length === 0 && (
				<p className="p-4 text-xs text-muted-foreground">{t("Ninguna carta coincide.")}</p>
			)}
		</div>
	);
	const listaAvisos = (
		<ul className="flex flex-col gap-0.5 p-1 text-xs">
			{avisos.map((a, i) => (
				<li
					key={`${a.tipo}-${i}`}
					className={cn(
						"flex items-center gap-1.5",
						impide(a) ? "text-destructive" : "text-amber-600 dark:text-amber-400",
					)}
				>
					<Icono icon="lucide:alert-triangle" className="size-3.5 shrink-0" />
					{textoAviso(a, t)}
				</li>
			))}
		</ul>
	);
	const cuenta = (
		<span
			className={cn(
				"shrink-0 rounded-full px-2 py-0.5 text-xs font-semibold tabular-nums",
				pide !== null && n !== pide
					? "bg-amber-500/20 text-amber-600 dark:text-amber-300"
					: "bg-primary/15 text-primary",
			)}
		>
			{pide !== null ? `${n} / ${pide}` : t("{n} cartas", { n })}
		</span>
	);
	const textoBloquea = t("Se puede guardar a medias, pero así no deja empezar la partida.");
	const botonGuardar = (
		<Button
			className={cn(movil && "min-w-0 flex-1")}
			onClick={() => void guardar()}
			disabled={!todas || !nombre.trim() || guardando}
		>
			{guardando ? (
				<Icono icon="lucide:loader-2" className="size-4 animate-spin" />
			) : (
				<Icono icon="lucide:check" className="size-4" />
			)}
			{t("Guardar")}
		</Button>
	);
	const botonCancelar = (
		<Button
			variant="outline"
			className={cn(movil && "min-w-0 flex-1")}
			onClick={onCerrar}
			disabled={guardando}
		>
			{t("Cancelar")}
		</Button>
	);
	const cabecera = (
		<DialogHeader>
			<DialogTitle>
				{lista ? t("Editar la lista para jugar") : t("Nueva lista para jugar")}
			</DialogTitle>
			<DialogDescription className={cn(movil && "sr-only")}>
				{commander
					? t("Elige el comandante (la variante que quieras) y 99 cartas más. El deck no cambia.")
					: t("Elige las cartas que se juegan. El deck no cambia.")}
			</DialogDescription>
		</DialogHeader>
	);
	const leyendo = (
		<div className="flex flex-1 items-center justify-center gap-2 text-xs text-muted-foreground">
			<Icono icon="lucide:loader-2" className="size-4 animate-spin" />
			{t("Leyendo las cartas del deck…")}
		</div>
	);

	// ── El móvil
	if (movil) {
		// De lado con el teclado quedan unos 120 dp: solo los campos y las cartas.
		const sinTecladoDeLado = "landscape:[[data-teclado]_&]:hidden";
		const pie = (
			<DialogFooter className={cn("!flex-row items-center gap-1.5", sinTecladoDeLado)}>
				{lista && (
					<Button
						size="icon"
						variant="outline"
						className="shrink-0"
						aria-label={t("Borrar")}
						onClick={() => void borrar()}
						disabled={guardando}
					>
						<Icono icon="lucide:trash-2" className="size-4" />
					</Button>
				)}
				{botonCancelar}
				{botonGuardar}
			</DialogFooter>
		);
		const botonAvisos = avisos.length > 0 && (
			<Button
				variant="outline"
				className={cn(
					"w-full shrink-0",
					sinTecladoDeLado,
					bloquea ? "text-destructive" : "text-amber-600 dark:text-amber-400",
				)}
				onClick={() => setVerAvisos(true)}
			>
				<Icono icon="lucide:alert-triangle" className="size-4" />
				{t("Avisos")} ({avisos.length})
			</Button>
		);
		const controles = todas && (
			<>
				<Input
					value={nombre}
					onChange={(e) => setNombre(e.target.value)}
					placeholder={t("Nombre de la lista")}
					aria-label={t("Nombre de la lista")}
					maxLength={60}
					className="h-10 w-full shrink-0 text-sm"
				/>
				<div className="flex shrink-0 items-center gap-1">
					<div className="relative min-w-0 flex-1">
						<Icono
							icon="lucide:search"
							className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
						/>
						<Input
							value={filtro}
							onChange={(e) => setFiltro(e.target.value)}
							placeholder={t("Nombre o tipo…")}
							className="h-10 pl-8 text-sm"
						/>
					</div>
					<Button
						size="icon-sm"
						variant={soloElegidas ? "secondary" : "ghost"}
						aria-label={t("Solo las elegidas")}
						aria-pressed={soloElegidas}
						onClick={() => setSoloElegidas((v) => !v)}
					>
						<Icono icon="lucide:list-checks" />
					</Button>
					<BotonMenu etiqueta={t("Más acciones")} titulo={t("Lista para jugar")}>
						<FilaMenu
							icono="lucide:list-restart"
							texto={t("Las del deck")}
							onClick={() => setEleccion(eleccionDeFabrica(todas))}
						/>
						<FilaMenu
							icono="lucide:eraser"
							texto={t("Ninguna")}
							onClick={() => setEleccion({ comandantes: [], cartas: [] })}
						/>
					</BotonMenu>
				</div>
			</>
		);
		const zonaCartas = todas ? (
			<div className="flex min-h-0 min-w-0 flex-1 flex-col gap-1.5">
				{/* De lado, esta fila es la de la X de cerrar. */}
				<div
					className={cn(
						"flex shrink-0 items-center gap-1.5",
						lado && "min-h-10 pr-10",
						sinTecladoDeLado,
					)}
				>
					{commander && (
						<ToggleGroup
							type="single"
							variant="outline"
							value={pestana}
							onValueChange={(v) => (v === "comandante" || v === "cartas") && setPestana(v)}
							className="min-w-0 flex-1"
						>
							<ToggleGroupItem value="comandante" className="min-w-0 flex-1 gap-1 px-1 text-xs">
								<Icono icon="game-icons:crown" className="size-3.5" />
								<span className="truncate">{t("Comandante")}</span>
							</ToggleGroupItem>
							<ToggleGroupItem value="cartas" className="min-w-0 flex-1 px-1 text-xs">
								<span className="truncate">{t("Cartas")}</span>
							</ToggleGroupItem>
						</ToggleGroup>
					)}
					<span className={cn(!commander && "ml-auto")}>{cuenta}</span>
				</div>
				<ScrollArea className="min-h-0 flex-1 rounded-lg border">
					{commander && pestana === "comandante" ? rejillaComandantes : rejillaCartas}
				</ScrollArea>
			</div>
		) : (
			leyendo
		);
		return (
			<Dialog open={abierto} onOpenChange={(o) => !o && !guardando && onCerrar()}>
				<DialogoAdaptable
					className={cn("flex flex-col gap-2 py-2", lado && "flex-row")}
					cambiosSinGuardar={sinGuardar}
					ocupado={guardando}
				>
					{lado ? (
						<>
							<div className="flex min-h-0 w-64 shrink-0 flex-col gap-2">
								<div className={sinTecladoDeLado}>{cabecera}</div>
								{controles}
								<div className="mt-auto flex flex-col gap-2">
									{botonAvisos}
									{pie}
								</div>
							</div>
							{zonaCartas}
						</>
					) : (
						<>
							{cabecera}
							{controles}
							{zonaCartas}
							{botonAvisos}
							{pie}
						</>
					)}
					<HojaMenu
						abierta={verAvisos && avisos.length > 0}
						onCerrar={() => setVerAvisos(false)}
						titulo={t("Avisos")}
					>
						{listaAvisos}
						{bloquea && <p className="text-xs text-muted-foreground">{textoBloquea}</p>}
					</HojaMenu>
				</DialogoAdaptable>
			</Dialog>
		);
	}

	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && !guardando && onCerrar()}>
			<DialogoAdaptable
				className="flex h-[88vh] flex-col sm:max-w-6xl"
				cambiosSinGuardar={sinGuardar}
				ocupado={guardando}
			>
				{cabecera}
				{!todas ? (
					leyendo
				) : (
					<>
						<div className="flex flex-wrap items-center gap-2.5">
							<Input
								value={nombre}
								onChange={(e) => setNombre(e.target.value)}
								placeholder={t("Nombre de la lista")}
								maxLength={60}
								className="h-8 w-64 text-xs"
							/>
							{cuenta}
							<div className="relative ml-auto">
								<Icono
									icon="lucide:search"
									className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground"
								/>
								<Input
									value={filtro}
									onChange={(e) => setFiltro(e.target.value)}
									placeholder={t("Nombre o tipo…")}
									className="h-8 w-52 pl-7 text-xs"
								/>
							</div>
							<label className="flex items-center gap-1.5 text-xs">
								<Switch checked={soloElegidas} onCheckedChange={setSoloElegidas} />
								{t("Solo las elegidas")}
							</label>
							<Button
								size="sm"
								variant="outline"
								onClick={() => setEleccion(eleccionDeFabrica(todas))}
							>
								{t("Las del deck")}
							</Button>
							<Button
								size="sm"
								variant="outline"
								onClick={() => setEleccion({ comandantes: [], cartas: [] })}
							>
								{t("Ninguna")}
							</Button>
						</div>
						<div className="flex min-h-0 flex-1 gap-2.5">
							{commander && (
								<section className="flex w-56 shrink-0 flex-col gap-1.5 rounded-lg border p-2">
									<h4 className="flex items-center gap-1.5 text-xs font-semibold">
										<Icono icon="game-icons:crown" className="size-3.5 text-amber-500" />
										{t("Comandante")}
									</h4>
									<ScrollArea className="min-h-0 flex-1">{rejillaComandantes}</ScrollArea>
								</section>
							)}
							<ScrollArea className="min-h-0 flex-1 rounded-lg border">{rejillaCartas}</ScrollArea>
						</div>
						{avisos.length > 0 && (
							<ScrollArea viewportClassName="max-h-24">{listaAvisos}</ScrollArea>
						)}
					</>
				)}
				<DialogFooter className="items-center">
					{bloquea && (
						<span className="mr-auto text-[11px] text-muted-foreground">{textoBloquea}</span>
					)}
					{lista && (
						<Button variant="outline" onClick={() => void borrar()} disabled={guardando}>
							<Icono icon="lucide:trash-2" className="size-4" />
							{t("Borrar")}
						</Button>
					)}
					{botonCancelar}
					{botonGuardar}
				</DialogFooter>
			</DialogoAdaptable>
		</Dialog>
	);
}
