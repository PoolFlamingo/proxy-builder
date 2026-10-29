import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaBanquillo } from "@/bindings/VistaBanquillo";
import { Icono } from "@/components/icono";
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
import { ScrollArea } from "@/components/ui/scroll-area";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import {
	agrupar,
	ajustar,
	type CambioDeJugador,
	type Cuantas,
	cambioValido,
	type GrupoCartas,
	hayCambio,
	idsDe,
	SIN_CAMBIOS,
	total,
	useEncuentro,
} from "./encuentro";

type Datos = { id: IdJugador; mazo: GrupoCartas[]; banquillo: GrupoCartas[] };

const agrupado = (b: VistaBanquillo, id: IdJugador): Datos => ({
	id,
	mazo: agrupar(b.mazo),
	banquillo: agrupar(b.banquillo),
});

/**
 * **El banquillo entre dos partidas** de un encuentro: cada jugador cambia
 * cartas del mazo por cartas de su banquillo (siempre las mismas que salen
 * que entran) y, al empezar la siguiente partida, Rust monta los mazos
 * nuevos. En una partida local se cambia el de cada uno; en red cada uno
 * cambia solo el suyo (el invitado se lo manda al anfitrión, que tiene los
 * mazos de todos) y la siguiente partida la empieza el anfitrión.
 */
export function BanquilloDialog({ onCerrar }: { onCerrar: () => void }) {
	const { t, mensaje } = useLang();
	const encuentro = useEncuentro();
	const todos = useMesa((s) => s.asientos);
	const yo = useMesa((s) => s.yo);
	const jugadores = useMesa((s) => s.jugadores);
	const papel = useRed((s) => s.papel);
	// En red, cada uno solo toca su mazo.
	const asientos = useMemo(
		() => (papel !== null && yo !== null ? [yo] : todos),
		[papel, yo, todos],
	);
	const invitado = papel === "jugador";
	const [datos, setDatos] = useState<Datos[]>([]);
	const [cambios, setCambios] = useState<Record<number, CambioDeJugador>>({});
	const [actual, setActual] = useState<IdJugador>(asientos[0] ?? 0);
	const [empezando, setEmpezando] = useState(false);

	useEffect(() => {
		let vivo = true;
		Promise.all(
			asientos.map(async (id) => agrupado(await call("mesa_banquillo", { jugador: id }), id)),
		)
			.then((d) => vivo && setDatos(d))
			.catch((e) => {
				if (vivo) toast.error(mensaje(e));
			});
		return () => {
			vivo = false;
		};
	}, [asientos, mensaje]);

	const de = datos.find((d) => d.id === actual);
	const cambio = cambios[actual] ?? SIN_CAMBIOS;
	const todoValido = useMemo(() => Object.values(cambios).every(cambioValido), [cambios]);
	const jugadas = encuentro?.estado?.jugadas ?? 0;
	const siguiente = jugadas + 1;

	const tocar = (lado: "sale" | "entra", nombre: string, delta: number, max: number) =>
		setCambios((c) => {
			const previo = c[actual] ?? SIN_CAMBIOS;
			return {
				...c,
				[actual]: { ...previo, [lado]: ajustar(previo[lado], nombre, delta, max) },
			};
		});

	const empezar = async () => {
		setEmpezando(true);
		try {
			for (const d of datos) {
				const c = cambios[d.id];
				if (!c || !hayCambio(c)) continue;
				await call("mesa_banquillo_cambiar", {
					jugador: d.id,
					sale: idsDe(d.mazo, c.sale),
					entra: idsDe(d.banquillo, c.entra),
				});
			}
			// El invitado solo deja su cambio: la partida la empieza el anfitrión.
			if (!invitado) await call("mesa_siguiente_partida");
			onCerrar();
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setEmpezando(false);
		}
	};

	return (
		<Dialog open onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-2xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Banquillo")}</DialogTitle>
					<DialogDescription>
						{t(
							"Cambia cartas del mazo por cartas del banquillo: tienen que salir tantas como entran.",
						)}
					</DialogDescription>
				</DialogHeader>
				{asientos.length > 1 && (
					<div className="flex flex-wrap gap-1.5">
						{asientos.map((id) => {
							const c = cambios[id] ?? SIN_CAMBIOS;
							return (
								<button
									key={id}
									type="button"
									onClick={() => setActual(id)}
									className={cn(
										"rounded-full border px-2.5 py-1 text-xs",
										id === actual && "border-primary bg-primary/10",
										!cambioValido(c) && "border-destructive text-destructive",
									)}
								>
									{jugadores[id]?.nombre ?? "?"}
									{hayCambio(c) && ` · ${total(c.sale)}`}
								</button>
							);
						})}
					</div>
				)}
				{de && (
					<div className="grid grid-cols-2 gap-2.5">
						<Columna
							titulo={t("Mazo ({cuantas})", {
								cuantas: de.mazo.reduce((a, g) => a + g.ids.length, 0),
							})}
							grupos={de.mazo}
							cuantas={cambio.sale}
							onTocar={(nombre, delta, max) => tocar("sale", nombre, delta, max)}
							etiqueta={(carta) => t("Sacar {carta} del mazo", { carta })}
						/>
						<Columna
							titulo={t("Banquillo ({cuantas})", {
								cuantas: de.banquillo.reduce((a, g) => a + g.ids.length, 0),
							})}
							grupos={de.banquillo}
							cuantas={cambio.entra}
							onTocar={(nombre, delta, max) => tocar("entra", nombre, delta, max)}
							etiqueta={(carta) => t("Meter {carta} en el mazo", { carta })}
							vacio={t("No hay cartas en el banquillo.")}
						/>
					</div>
				)}
				<p
					className={cn(
						"text-xs",
						cambioValido(cambio) ? "text-muted-foreground" : "text-destructive",
					)}
				>
					{cambioValido(cambio)
						? t("Salen {sale} y entran {entra}.", {
								sale: total(cambio.sale),
								entra: total(cambio.entra),
							})
						: t("Tienen que salir y entrar las mismas cartas ({sale} y {entra}).", {
								sale: total(cambio.sale),
								entra: total(cambio.entra),
							})}
				</p>
				{invitado && (
					<p className="text-xs text-muted-foreground">
						{t("El anfitrión empieza la siguiente partida.")}
					</p>
				)}
				<DialogFooter>
					<Button variant="outline" onClick={onCerrar}>
						{t("Cancelar")}
					</Button>
					<Button disabled={!todoValido || empezando} onClick={() => void empezar()}>
						{invitado ? (
							<Icono icon="lucide:check" className="size-4" />
						) : (
							<Icono icon="lucide:play" className="size-4" />
						)}
						{invitado ? t("Guardar") : t("Empezar la partida {siguiente}", { siguiente })}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function Columna({
	titulo,
	grupos,
	cuantas,
	onTocar,
	etiqueta,
	vacio,
}: {
	titulo: string;
	grupos: GrupoCartas[];
	cuantas: Cuantas;
	onTocar: (nombre: string, delta: number, max: number) => void;
	/** La etiqueta de los botones de una carta. */
	etiqueta: (nombre: string) => string;
	vacio?: string;
}) {
	return (
		<div className="flex min-w-0 flex-col gap-1.5">
			<h3 className="text-xs font-medium">{titulo}</h3>
			<ScrollArea className="h-72 rounded-lg border">
				<ul className="flex flex-col gap-0.5 p-1">
					{grupos.length === 0 && vacio && (
						<li className="p-2 text-xs text-muted-foreground">{vacio}</li>
					)}
					{grupos.map((g) => {
						const n = cuantas[g.nombre] ?? 0;
						return (
							<li
								key={g.nombre}
								className={cn(
									"flex items-center gap-1.5 rounded-md px-1.5 py-0.5 text-xs",
									n > 0 && "bg-primary/10",
								)}
							>
								<span className="min-w-0 flex-1 truncate">{g.nombre}</span>
								<span className="text-muted-foreground tabular-nums">×{g.ids.length}</span>
								<Paso
									etiqueta={etiqueta(g.nombre)}
									disabled={n === 0}
									onClick={() => onTocar(g.nombre, -1, g.ids.length)}
								>
									<Icono icon="lucide:minus" className="size-3" />
								</Paso>
								<b className="min-w-3 text-center tabular-nums">{n}</b>
								<Paso
									etiqueta={etiqueta(g.nombre)}
									disabled={n >= g.ids.length}
									onClick={() => onTocar(g.nombre, 1, g.ids.length)}
								>
									<Icono icon="lucide:plus" className="size-3" />
								</Paso>
							</li>
						);
					})}
				</ul>
			</ScrollArea>
		</div>
	);
}

function Paso({
	etiqueta,
	disabled,
	onClick,
	children,
}: {
	etiqueta: string;
	disabled: boolean;
	onClick: () => void;
	children: React.ReactNode;
}) {
	return (
		<button
			type="button"
			aria-label={etiqueta}
			disabled={disabled}
			onClick={onClick}
			className="flex size-5 shrink-0 items-center justify-center rounded-full border bg-background/50 hover:bg-accent disabled:opacity-30"
		>
			{children}
		</button>
	);
}
