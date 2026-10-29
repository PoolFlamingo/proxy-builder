import { useMemo, useState } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { Zona } from "@/bindings/Zona";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
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
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useMesa } from "@/stores/mesa-store";
import { destino, hacer, mover, sitio } from "../acciones";
import { caraVisible, ImagenCarta } from "../imagen-carta";
import { usePantalla } from "../pantalla";

export type FiltroBusqueda = {
	texto: string;
	tipo: "todo" | "tierra" | "criatura" | "noTierra" | "instantaneo" | "artefacto" | "encantamiento";
	color: "" | "W" | "U" | "B" | "R" | "G" | "C";
};

/** Las cartas que pasan el filtro del buscador (nombre, tipo y color). */
export function filtrar(cartas: VistaCarta[], f: FiltroBusqueda): VistaCarta[] {
	const texto = f.texto.trim().toLowerCase();
	return cartas.filter((c) => {
		const o = c.origen;
		if (!o) return false;
		const tipo = (o.info.tipo ?? "").toLowerCase();
		if (texto && !o.nombre.toLowerCase().includes(texto) && !tipo.includes(texto)) return false;
		switch (f.tipo) {
			case "tierra":
				if (!o.info.esTierra) return false;
				break;
			case "noTierra":
				if (o.info.esTierra) return false;
				break;
			case "criatura":
				if (!o.info.esCriatura) return false;
				break;
			case "instantaneo":
				if (!tipo.includes("instant") && !tipo.includes("sorcery")) return false;
				break;
			case "artefacto":
				if (!tipo.includes("artifact")) return false;
				break;
			case "encantamiento":
				if (!tipo.includes("enchantment")) return false;
				break;
		}
		if (f.color === "C" && o.info.colores.length > 0) return false;
		if (f.color && f.color !== "C" && !o.info.colores.includes(f.color)) return false;
		return true;
	});
}

/**
 * Las cartas que se han puesto arriba de la biblioteca (en orden, la última
 * arriba del todo) tras mover `id` a `zona`: se añade si va arriba y se quita
 * si va a otro sitio.
 */
export function arribaTrasSacar(arriba: IdCarta[], id: IdCarta, zona: Zona): IdCarta[] {
	const sin = arriba.filter((x) => x !== id);
	return zona === "biblioteca" ? [...sin, id] : sin;
}

const TIPOS: { tipo: FiltroBusqueda["tipo"]; nombre: string }[] = [
	{ tipo: "todo", nombre: "Todo" },
	{ tipo: "tierra", nombre: "Tierras" },
	{ tipo: "noTierra", nombre: "No tierras" },
	{ tipo: "criatura", nombre: "Criaturas" },
	{ tipo: "instantaneo", nombre: "Instantáneos y conjuros" },
	{ tipo: "artefacto", nombre: "Artefactos" },
	{ tipo: "encantamiento", nombre: "Encantamientos" },
];

const COLORES_FILTRO: FiltroBusqueda["color"][] = ["", "W", "U", "B", "R", "G", "C"];

/**
 * Una rejilla de cartas con buscador, filtros y botones por carta (a la
 * mano, al campo…). Para buscar en la biblioteca y para ver el cementerio,
 * el exilio, el mando o el banquillo.
 */
function Rejilla({
	cartas,
	filtroInicial,
	acciones,
	separadorAntes,
}: {
	cartas: VistaCarta[];
	filtroInicial: FiltroBusqueda["tipo"];
	acciones: (c: VistaCarta) => React.ReactNode;
	/** Con el separador «Antes»: lo que llegó en rondas anteriores (cementerio, exilio). */
	separadorAntes?: number;
}) {
	const { t } = useLang();
	const [f, setF] = useState<FiltroBusqueda>({ texto: "", tipo: filtroInicial, color: "" });
	const visibles = useMemo(() => filtrar(cartas, f), [cartas, f]);
	const ahora =
		separadorAntes !== undefined
			? visibles.filter((c) => c.llegoEnRonda >= separadorAntes)
			: visibles;
	const antes =
		separadorAntes !== undefined ? visibles.filter((c) => c.llegoEnRonda < separadorAntes) : [];
	const pinta = (lista: VistaCarta[]) =>
		lista.map((c) => (
			<div key={c.id} className="flex w-32 flex-col gap-1">
				<div
					className="h-[178px] overflow-hidden rounded-[5%]"
					onPointerEnter={() => useMesa.getState().setSobre(c.id)}
					onPointerLeave={() => useMesa.getState().setSobre(null)}
				>
					<ImagenCarta imagen={caraVisible(c, true)} ancho={244} nombre={c.origen?.nombre} />
				</div>
				<div className="flex flex-wrap justify-center gap-1">{acciones(c)}</div>
			</div>
		));
	return (
		<div className="flex min-h-0 flex-col gap-2.5">
			<div className="flex flex-wrap items-center gap-1.5" data-filtro="">
				<div className="relative">
					<Icono
						icon="lucide:search"
						className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground"
					/>
					<Input
						autoFocus
						value={f.texto}
						onChange={(e) => setF({ ...f, texto: e.target.value })}
						placeholder={t("Nombre o tipo…")}
						className="h-7 w-48 pl-7 text-xs"
					/>
				</div>
				{TIPOS.map((x) => (
					<Button
						key={x.tipo}
						size="sm"
						variant={f.tipo === x.tipo ? "default" : "outline"}
						className="h-7 px-2 text-[11px]"
						onClick={() => setF({ ...f, tipo: x.tipo })}
					>
						{t(x.nombre)}
					</Button>
				))}
				{COLORES_FILTRO.map((col) => (
					<Button
						key={col || "todos"}
						size="sm"
						variant={f.color === col ? "default" : "outline"}
						className="h-7 w-7 px-0 text-[11px]"
						onClick={() => setF({ ...f, color: col })}
					>
						{col || "∗"}
					</Button>
				))}
			</div>
			<ScrollArea viewportClassName="h-[58vh]">
				<div className="flex flex-wrap gap-2.5 p-1">
					{pinta(ahora)}
					{antes.length > 0 && (
						<>
							<div className="flex w-full items-center gap-2 text-[11px] text-muted-foreground">
								<span className="h-px flex-1 bg-border" />
								{t("Antes")}
								<span className="h-px flex-1 bg-border" />
							</div>
							{pinta(antes)}
						</>
					)}
					{visibles.length === 0 && (
						<p className="p-4 text-xs text-muted-foreground">{t("Ninguna carta coincide.")}</p>
					)}
				</div>
			</ScrollArea>
		</div>
	);
}

function BotonCarta({
	ayuda,
	onClick,
	children,
}: {
	ayuda: string;
	onClick: () => void;
	children: React.ReactNode;
}) {
	return (
		<WithTooltip content={ayuda}>
			<Button size="icon" variant="outline" className="size-6" onClick={onClick} aria-label={ayuda}>
				{children}
			</Button>
		</WithTooltip>
	);
}

/** Buscar en la biblioteca (tutor, buscar tierra, ver la biblioteca). Al cerrar, ofrece barajar. */
export function BuscarDialog({ de, filtro }: { de: IdJugador; filtro: "todo" | "tierra" }) {
	const { t } = useLang();
	const lib = useMesa((s) => s.zonas[`${de}:biblioteca`]);
	const todas = useMesa((s) => s.cartas);
	const [sacada, setSacada] = useState(false);
	// Lo que se ha puesto arriba mientras está abierto, en el orden en que se puso
	// (la última queda arriba del todo): barajar no lo deshace.
	const [arriba, setArriba] = useState<IdCarta[]>([]);
	const cartas = useMemo(
		() =>
			(lib?.visibles ?? [])
				.sort(([a], [b]) => a - b)
				.map(([, id]) => todas[id])
				.filter((c): c is VistaCarta => !!c),
		[lib, todas],
	);
	const sacar = (id: IdCarta, zona: Zona, s = sitio.arriba) => {
		setSacada(true);
		setArriba((a) => arribaTrasSacar(a, id, zona));
		void mover([id], destino(zona, s, zona === "campo" ? de : null));
	};
	const cerrar = (barajar: boolean) => {
		usePantalla.getState().abrir(null);
		if (!barajar) return void hacer({ que: "dejarDeMirar", de });
		// En una partida de verdad se baraja y luego se pone la carta arriba:
		// aquí, lo mismo, aunque se haya puesto antes de barajar.
		void (async () => {
			if (!(await hacer({ que: "barajar" })) || arriba.length === 0) return;
			await mover(arriba, destino("biblioteca", sitio.arriba));
		})();
	};
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar(false)}>
			<DialogContent className="sm:max-w-5xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Biblioteca")}</DialogTitle>
					<DialogDescription>{t("{n} cartas", { n: cartas.length })}</DialogDescription>
				</DialogHeader>
				<Rejilla
					cartas={cartas}
					filtroInicial={filtro === "tierra" ? "tierra" : "todo"}
					acciones={(c) => (
						<>
							<BotonCarta ayuda={t("Mano")} onClick={() => sacar(c.id, "mano")}>
								<Icono icon="lucide:hand" className="size-3" />
							</BotonCarta>
							<BotonCarta ayuda={t("Campo")} onClick={() => sacar(c.id, "campo")}>
								<Icono icon="lucide:layout-grid" className="size-3" />
							</BotonCarta>
							<BotonCarta
								ayuda={t("Arriba")}
								onClick={() => sacar(c.id, "biblioteca", sitio.arriba)}
							>
								<Icono icon="lucide:arrow-up-to-line" className="size-3" />
							</BotonCarta>
							<BotonCarta ayuda={t("Cementerio")} onClick={() => sacar(c.id, "cementerio")}>
								<Icono icon="game-icons:tombstone" className="size-3" />
							</BotonCarta>
						</>
					)}
				/>
				{arriba.length > 0 && (
					<p className="text-xs text-muted-foreground">
						{t("Al barajar, las cartas que has puesto arriba vuelven a quedar arriba.")}
					</p>
				)}
				<DialogFooter>
					<Button variant="outline" onClick={() => cerrar(false)}>
						{t("Cerrar sin barajar")}
					</Button>
					<Button onClick={() => cerrar(true)} variant={sacada ? "default" : "secondary"}>
						{t("Barajar y cerrar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

const NOMBRES_ZONA: Partial<Record<Zona, string>> = {
	cementerio: paraTraducir("Cementerio"),
	exilio: paraTraducir("Exilio"),
	mando: paraTraducir("Mando"),
	banquillo: paraTraducir("Banquillo"),
};

/** Ver el cementerio, el exilio, el mando o el banquillo, con el separador «Antes». */
/**
 * **Ver una zona entera** sin sacar las cartas una a una. De cementerio y
 * exilio hay pestañas para pasar de una a otra y, con varios jugadores, un
 * selector con las de cada uno (y cuántas lleva cada cual): se abre desde la
 * barra, el menú de la zona o con doble clic en ella.
 */
export function VerZonaDialog({ jugador, zona }: { jugador: IdJugador; zona: Zona }) {
	const { t } = useLang();
	const [de, setDe] = useState(jugador);
	const [cual, setCual] = useState(zona);
	const v = useMesa((s) => s.zonas[`${de}:${cual}`]);
	const todas = useMesa((s) => s.cartas);
	const jugadores = useMesa((s) => s.jugadores);
	const zonas = useMesa((s) => s.zonas);
	const ronda = useMesa((s) => s.general?.turno.ronda ?? 0);
	const cartas = useMemo(
		() =>
			[...(v?.cartas ?? [])]
				.reverse()
				.map((id) => todas[id])
				.filter((c): c is VistaCarta => !!c),
		[v, todas],
	);
	const pestanas = zona === "cementerio" || zona === "exilio";
	const cuantas = (j: IdJugador, z: Zona) => zonas[`${j}:${z}`]?.cuantas ?? 0;
	const lista = Object.values(jugadores);
	// El campo es de quien mueve: robar del cementerio de otro lo lleva al propio.
	const a = (id: IdCarta, z: Zona, s = sitio.arriba) => void mover([id], destino(z, s, null));
	return (
		<Dialog open onOpenChange={(o) => !o && usePantalla.getState().abrir(null)}>
			<DialogContent className="sm:max-w-5xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>
						{NOMBRES_ZONA[cual] ? t(NOMBRES_ZONA[cual]) : t("Zona")}
						{lista.length > 1 && ` · ${jugadores[de]?.nombre ?? ""}`}
					</DialogTitle>
					<DialogDescription>{t("{n} cartas", { n: cartas.length })}</DialogDescription>
				</DialogHeader>
				{pestanas && (
					<div className="flex flex-wrap items-center gap-1.5">
						{(["cementerio", "exilio"] as const).map((z) => (
							<Button
								key={z}
								size="sm"
								variant={cual === z ? "default" : "outline"}
								onClick={() => setCual(z)}
							>
								{z === "cementerio" ? (
									<Icono icon="game-icons:tombstone" />
								) : (
									<Icono icon="lucide:trash-2" />
								)}
								{t(NOMBRES_ZONA[z] ?? "")} · {cuantas(de, z)}
							</Button>
						))}
						{lista.length > 1 && <span className="mx-1 h-5 w-px bg-border" />}
						{lista.length > 1 &&
							lista.map((j) => (
								<Button
									key={j.id}
									size="sm"
									variant={de === j.id ? "default" : "outline"}
									onClick={() => setDe(j.id)}
								>
									<span className="size-2.5 rounded-full" style={{ background: j.color }} />
									{j.nombre} · {cuantas(j.id, cual)}
								</Button>
							))}
					</div>
				)}
				<Rejilla
					cartas={cartas}
					filtroInicial="todo"
					separadorAntes={cual === "cementerio" || cual === "exilio" ? ronda : undefined}
					acciones={(c) => (
						<>
							<BotonCarta ayuda={t("Mano")} onClick={() => a(c.id, "mano")}>
								<Icono icon="lucide:hand" className="size-3" />
							</BotonCarta>
							<BotonCarta ayuda={t("Campo")} onClick={() => a(c.id, "campo")}>
								<Icono icon="lucide:layout-grid" className="size-3" />
							</BotonCarta>
							{cual !== "cementerio" && (
								<BotonCarta ayuda={t("Cementerio")} onClick={() => a(c.id, "cementerio")}>
									<Icono icon="game-icons:tombstone" className="size-3" />
								</BotonCarta>
							)}
							{cual !== "exilio" && (
								<BotonCarta ayuda={t("Exilio")} onClick={() => a(c.id, "exilio")}>
									<Icono icon="lucide:trash-2" className="size-3" />
								</BotonCarta>
							)}
							<BotonCarta ayuda={t("Abajo")} onClick={() => a(c.id, "biblioteca", sitio.abajo)}>
								<Icono icon="lucide:arrow-down-to-line" className="size-3" />
							</BotonCarta>
						</>
					)}
				/>
			</DialogContent>
		</Dialog>
	);
}
