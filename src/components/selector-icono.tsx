import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColeccionIconos } from "@/bindings/ColeccionIconos";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useIconos } from "@/stores/iconos-store";

/** «Todas»: buscar por nombre en todas las colecciones. */
const TODAS = "*";
/** Iconos que se pintan de una vez (y cada «Más»). */
const PAGINA = 240;
/** La colección con la que se abre si no hay icono: la de los juegos. */
const DE_FABRICA = "game-icons";

// Lo que ya trajo Rust en esta sesión (Rust además lo guarda en disco).
let colecciones: ColeccionIconos[] | null = null;
const nombres = new Map<string, string[]>();
const busquedas = new Map<string, string[]>();

/**
 * **Elegir cualquier icono de Iconify** (las más de 200 colecciones): una
 * colección o «Todas» (buscar por nombre en todas) y un filtro. Devuelve el id
 * entero (`game-icons:crown`) y Rust lo guarda para que se vea sin red
 * (`iconos_elegir`). Como el `IconPicker` de la web, pero los datos los pide
 * Rust: la vista no sale a internet.
 */
export function SelectorIcono({
	valor,
	onChange,
	quitable = false,
	className,
	disabled,
}: {
	valor: string | null;
	onChange: (id: string | null) => void;
	quitable?: boolean;
	className?: string;
	disabled?: boolean;
}) {
	const { t, mensaje } = useLang();
	const [abierto, setAbierto] = useState(false);
	const [lista, setLista] = useState<ColeccionIconos[]>(colecciones ?? []);
	const [coleccion, setColeccion] = useState(valor?.split(":")[0] ?? DE_FABRICA);
	const [eligiendoColeccion, setEligiendoColeccion] = useState(false);
	const [filtroColeccion, setFiltroColeccion] = useState("");
	const [filtro, setFiltro] = useState("");
	const [ids, setIds] = useState<string[]>([]);
	const [cargando, setCargando] = useState(false);
	const [error, setError] = useState<unknown>(null);
	const [cuantos, setCuantos] = useState(PAGINA);
	const todas = coleccion === TODAS;

	useEffect(() => {
		if (!abierto || colecciones) return;
		let vivo = true;
		call("iconos_colecciones")
			.then((c) => {
				colecciones = c;
				if (vivo) setLista(c);
			})
			.catch((e) => vivo && setError(e));
		return () => {
			vivo = false;
		};
	}, [abierto]);

	// Los ids de la colección (filtrados aquí) o de la búsqueda (en Rust).
	useEffect(() => {
		if (!abierto) return;
		let vivo = true;
		setCuantos(PAGINA);
		setError(null);
		const q = filtro.trim().toLowerCase();
		if (todas) {
			if (q.length < 2) {
				setIds([]);
				return;
			}
			const hecha = busquedas.get(q);
			if (hecha) {
				setIds(hecha);
				return;
			}
			setCargando(true);
			const espera = window.setTimeout(() => {
				call("iconos_buscar", { texto: q })
					.then((r) => {
						busquedas.set(q, r);
						if (vivo) setIds(r);
					})
					.catch((e) => vivo && setError(e))
					.finally(() => vivo && setCargando(false));
			}, 250);
			return () => {
				vivo = false;
				window.clearTimeout(espera);
			};
		}
		const filtrar = (n: string[]) =>
			(q ? n.filter((x) => x.includes(q)) : n).map((x) => `${coleccion}:${x}`);
		const hechos = nombres.get(coleccion);
		if (hechos) {
			setIds(filtrar(hechos));
			return;
		}
		setCargando(true);
		call("iconos_nombres", { prefijo: coleccion })
			.then((n) => {
				nombres.set(coleccion, n);
				if (vivo) setIds(filtrar(n));
			})
			.catch((e) => vivo && setError(e))
			.finally(() => vivo && setCargando(false));
		return () => {
			vivo = false;
		};
	}, [abierto, coleccion, filtro, todas]);

	const actual = lista.find((c) => c.prefijo === coleccion);
	const filtradas = useMemo(() => {
		const q = filtroColeccion.trim().toLowerCase();
		return q
			? lista.filter((c) => c.nombre.toLowerCase().includes(q) || c.prefijo.includes(q))
			: lista;
	}, [lista, filtroColeccion]);

	const elegir = (id: string) => {
		call("iconos_elegir", { id })
			.then((d) => {
				useIconos.getState().recibir({ [id]: d });
				onChange(id);
				setAbierto(false);
			})
			.catch((e) => toast.error(mensaje(e)));
	};

	const vistos = ids.slice(0, cuantos);
	return (
		<Popover open={abierto} onOpenChange={setAbierto}>
			<WithTooltip content={valor ?? t("Elegir un icono")}>
				<PopoverTrigger asChild disabled={disabled}>
					<Button
						type="button"
						variant="outline"
						size="icon"
						className={cn("shrink-0", className)}
						disabled={disabled}
						aria-label={t("Elegir un icono")}
					>
						<Icono icon={valor ?? "lucide:image-plus"} className={cn(!valor && "opacity-50")} />
					</Button>
				</PopoverTrigger>
			</WithTooltip>
			<PopoverContent
				align="start"
				className="w-[min(95vw,23rem)] p-2.5 text-xs"
				onOpenAutoFocus={(e) => e.preventDefault()}
			>
				<div className="flex flex-col gap-2.5">
					<Popover open={eligiendoColeccion} onOpenChange={setEligiendoColeccion}>
						<PopoverTrigger asChild>
							<Button variant="outline" size="sm" className="justify-between">
								<span className="truncate">
									{todas ? t("Todas las colecciones") : (actual?.nombre ?? coleccion)}
									{!todas && actual && (
										<span className="text-muted-foreground"> · {actual.total}</span>
									)}
								</span>
								<Icono icon="lucide:chevrons-up-down" className="opacity-60" />
							</Button>
						</PopoverTrigger>
						<PopoverContent align="start" className="w-[min(90vw,20rem)] p-2.5 text-xs">
							<Input
								autoFocus
								value={filtroColeccion}
								onChange={(e) => setFiltroColeccion(e.target.value)}
								placeholder={t("Buscar colección…")}
							/>
							<ScrollArea className="mt-2.5 h-72">
								<div className="flex flex-col p-1">
									{[{ prefijo: TODAS, nombre: t("Todas las colecciones"), total: 0 }]
										.concat(filtradas)
										.map((c) => (
											<button
												key={c.prefijo}
												type="button"
												className={cn(
													"flex items-center justify-between gap-2.5 rounded-md px-2 py-1 text-left hover:bg-accent",
													c.prefijo === coleccion && "bg-accent",
												)}
												onClick={() => {
													setColeccion(c.prefijo);
													setEligiendoColeccion(false);
												}}
											>
												<span className="truncate">{c.nombre}</span>
												{c.total > 0 && (
													<span className="shrink-0 text-muted-foreground tabular-nums">
														{c.total}
													</span>
												)}
											</button>
										))}
								</div>
							</ScrollArea>
						</PopoverContent>
					</Popover>
					<Input
						autoFocus
						value={filtro}
						onChange={(e) => setFiltro(e.target.value)}
						placeholder={todas ? t("Buscar en todas (2 letras o más)…") : t("Filtrar por nombre…")}
					/>
					<ScrollArea className="h-64">
						<div className="grid grid-cols-8 gap-1 p-1">
							{vistos.map((id) => (
								<WithTooltip key={id} content={id}>
									<button
										type="button"
										className={cn(
											"flex aspect-square items-center justify-center rounded-md hover:bg-accent",
											id === valor && "bg-primary/15 ring-2 ring-primary",
										)}
										onClick={() => elegir(id)}
									>
										<Icono icon={id} className="size-5" />
									</button>
								</WithTooltip>
							))}
						</div>
						{ids.length > cuantos && (
							<div className="p-1">
								<Button
									variant="ghost"
									size="sm"
									className="w-full"
									onClick={() => setCuantos((n) => n + PAGINA)}
								>
									{t("Mostrar más")}
								</Button>
							</div>
						)}
						{vistos.length === 0 && (
							<p className="p-2.5 text-center text-muted-foreground">
								{error
									? mensaje(error)
									: cargando
										? t("Cargando…")
										: todas && filtro.trim().length < 2
											? t("Escribe para buscar en todas las colecciones.")
											: t("Ningún icono con ese nombre.")}
							</p>
						)}
					</ScrollArea>
					{quitable && valor && (
						<Button
							variant="ghost"
							size="sm"
							onClick={() => {
								onChange(null);
								setAbierto(false);
							}}
						>
							<Icono icon="lucide:x" />
							{t("Sin icono")}
						</Button>
					)}
				</div>
			</PopoverContent>
		</Popover>
	);
}
