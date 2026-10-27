import { AlertTriangle, Check, Crown, Loader2, Search, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { AvisoLista } from "@/bindings/AvisoLista";
import type { CartaDeLista } from "@/bindings/CartaDeLista";
import type { Formato } from "@/bindings/Formato";
import type { ListaDeJuego } from "@/bindings/ListaDeJuego";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { PORTADA } from "@/features/library/vista-cuadricula";
import { useEstadoParaCerrar } from "@/lib/dialogos-abiertos";
import { urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
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
				setTodas(c);
				setNombre(lista?.nombre ?? "");
				setEleccion(
					lista ? { comandantes: lista.comandantes, cartas: lista.cartas } : eleccionDeFabrica(c),
				);
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

	const candidatos = useMemo(() => (todas ?? []).filter((c) => c.puedeSerComandante), [todas]);
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

	const miniatura = (c: CartaDeLista, elegida: boolean, alPulsar: () => void, corona = false) => (
		<button
			key={c.id}
			type="button"
			onClick={alPulsar}
			className={cn(
				"relative h-[105px] w-[75px] shrink-0 overflow-hidden rounded-[5%] bg-muted transition",
				elegida ? "ring-2 ring-primary" : "opacity-45 hover:opacity-80",
			)}
			aria-pressed={elegida}
			aria-label={c.nombre}
		>
			{c.imagen ? (
				<img
					src={urlMiniatura(c.imagen, PORTADA, 146)}
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
					{corona ? <Crown className="size-2.5" /> : <Check className="size-2.5" />}
				</span>
			)}
			{c.marca && MARCAS[c.marca] && (
				<span className="absolute inset-x-0 bottom-0 bg-violet-600/85 text-center text-[8px] text-white">
					{t(MARCAS[c.marca])}
				</span>
			)}
		</button>
	);

	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && !guardando && onCerrar()}>
			<DialogContent
				className="flex h-[88vh] flex-col sm:max-w-6xl"
				cambiosSinGuardar={sinGuardar}
				ocupado={guardando}
			>
				<DialogHeader>
					<DialogTitle>
						{lista ? t("Editar la lista para jugar") : t("Nueva lista para jugar")}
					</DialogTitle>
					<DialogDescription>
						{commander
							? t(
									"Elige el comandante (la variante que quieras) y 99 cartas más. El deck no cambia.",
								)
							: t("Elige las cartas que se juegan. El deck no cambia.")}
					</DialogDescription>
				</DialogHeader>
				{!todas ? (
					<div className="flex flex-1 items-center justify-center gap-2 text-xs text-muted-foreground">
						<Loader2 className="size-4 animate-spin" />
						{t("Leyendo las cartas del deck…")}
					</div>
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
							<span
								className={cn(
									"rounded-full px-2 py-0.5 text-xs font-semibold tabular-nums",
									pide !== null && n !== pide
										? "bg-amber-500/20 text-amber-600 dark:text-amber-300"
										: "bg-primary/15 text-primary",
								)}
							>
								{pide !== null ? `${n} / ${pide}` : t("{n} cartas", { n })}
							</span>
							<div className="relative ml-auto">
								<Search className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
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
										<Crown className="size-3.5 text-amber-500" />
										{t("Comandante")}
									</h4>
									<ScrollArea className="min-h-0 flex-1">
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
													() => setEleccion((e) => alternarComandante(e, c, todas)),
													true,
												),
											)}
										</div>
									</ScrollArea>
								</section>
							)}
							<ScrollArea className="min-h-0 flex-1 rounded-lg border">
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
														setEleccion((e) =>
															conCopias(e, copias, elegidas > 0 ? 0 : copias.length),
														),
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
													<span className="w-[75px] truncate text-center text-[10px]">
														{nombreCarta}
													</span>
												</div>
											);
										return (
											<div key={nombreCarta} className="flex flex-col gap-1">
												<div className="flex gap-1">
													{copias.map((c) =>
														miniatura(
															c,
															eleccion.cartas.includes(c.id) || eleccion.comandantes.includes(c.id),
															() => setEleccion((e) => alternarCarta(e, c, todas, singleton)),
															eleccion.comandantes.includes(c.id),
														),
													)}
												</div>
												<span
													className="truncate text-[10px]"
													style={{ maxWidth: copias.length * 79 - 4 }}
												>
													{nombreCarta}
												</span>
											</div>
										);
									})}
									{grupos.length === 0 && (
										<p className="p-4 text-xs text-muted-foreground">
											{t("Ninguna carta coincide.")}
										</p>
									)}
								</div>
							</ScrollArea>
						</div>
						{avisos.length > 0 && (
							<ScrollArea viewportClassName="max-h-24">
								<ul className="flex flex-col gap-0.5 p-1 text-xs">
									{avisos.map((a, i) => (
										<li
											key={`${a.tipo}-${i}`}
											className={cn(
												"flex items-center gap-1.5",
												impide(a) ? "text-destructive" : "text-amber-600 dark:text-amber-400",
											)}
										>
											<AlertTriangle className="size-3.5 shrink-0" />
											{textoAviso(a, t)}
										</li>
									))}
								</ul>
							</ScrollArea>
						)}
					</>
				)}
				<DialogFooter className="items-center">
					{bloquea && (
						<span className="mr-auto text-[11px] text-muted-foreground">
							{t("Se puede guardar a medias, pero así no deja empezar la partida.")}
						</span>
					)}
					{lista && (
						<Button variant="outline" onClick={() => void borrar()} disabled={guardando}>
							<Trash2 className="size-4" />
							{t("Borrar")}
						</Button>
					)}
					<Button variant="outline" onClick={onCerrar} disabled={guardando}>
						{t("Cancelar")}
					</Button>
					<Button onClick={() => void guardar()} disabled={!todas || !nombre.trim() || guardando}>
						{guardando ? <Loader2 className="size-4 animate-spin" /> : <Check className="size-4" />}
						{t("Guardar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
