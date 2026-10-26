import {
	ArrowDownToLine,
	ArrowUpToLine,
	Hand,
	LayoutGrid,
	Search,
	Skull,
	Trash2,
} from "lucide-react";
import { useMemo, useState } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { Zona } from "@/bindings/Zona";
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
					<Search className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
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
		void mover([id], destino(zona, s, zona === "campo" ? de : null));
	};
	const cerrar = (barajar: boolean) => {
		usePantalla.getState().abrir(null);
		void (barajar ? hacer({ que: "barajar" }) : hacer({ que: "dejarDeMirar", de }));
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
								<Hand className="size-3" />
							</BotonCarta>
							<BotonCarta ayuda={t("Campo")} onClick={() => sacar(c.id, "campo")}>
								<LayoutGrid className="size-3" />
							</BotonCarta>
							<BotonCarta
								ayuda={t("Arriba")}
								onClick={() => sacar(c.id, "biblioteca", sitio.arriba)}
							>
								<ArrowUpToLine className="size-3" />
							</BotonCarta>
							<BotonCarta ayuda={t("Cementerio")} onClick={() => sacar(c.id, "cementerio")}>
								<Skull className="size-3" />
							</BotonCarta>
						</>
					)}
				/>
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
export function VerZonaDialog({ jugador, zona }: { jugador: IdJugador; zona: Zona }) {
	const { t } = useLang();
	const v = useMesa((s) => s.zonas[`${jugador}:${zona}`]);
	const todas = useMesa((s) => s.cartas);
	const ronda = useMesa((s) => s.general?.turno.ronda ?? 0);
	const cartas = useMemo(
		() =>
			[...(v?.cartas ?? [])]
				.reverse()
				.map((id) => todas[id])
				.filter((c): c is VistaCarta => !!c),
		[v, todas],
	);
	const a = (id: IdCarta, z: Zona, s = sitio.arriba) =>
		void mover([id], destino(z, s, z === "campo" ? jugador : null));
	return (
		<Dialog open onOpenChange={(o) => !o && usePantalla.getState().abrir(null)}>
			<DialogContent className="sm:max-w-5xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{NOMBRES_ZONA[zona] ? t(NOMBRES_ZONA[zona]) : t("Zona")}</DialogTitle>
					<DialogDescription>{t("{n} cartas", { n: cartas.length })}</DialogDescription>
				</DialogHeader>
				<Rejilla
					cartas={cartas}
					filtroInicial="todo"
					separadorAntes={zona === "cementerio" || zona === "exilio" ? ronda : undefined}
					acciones={(c) => (
						<>
							<BotonCarta ayuda={t("Mano")} onClick={() => a(c.id, "mano")}>
								<Hand className="size-3" />
							</BotonCarta>
							<BotonCarta ayuda={t("Campo")} onClick={() => a(c.id, "campo")}>
								<LayoutGrid className="size-3" />
							</BotonCarta>
							{zona !== "cementerio" && (
								<BotonCarta ayuda={t("Cementerio")} onClick={() => a(c.id, "cementerio")}>
									<Skull className="size-3" />
								</BotonCarta>
							)}
							{zona !== "exilio" && (
								<BotonCarta ayuda={t("Exilio")} onClick={() => a(c.id, "exilio")}>
									<Trash2 className="size-3" />
								</BotonCarta>
							)}
							<BotonCarta ayuda={t("Abajo")} onClick={() => a(c.id, "biblioteca", sitio.abajo)}>
								<ArrowDownToLine className="size-3" />
							</BotonCarta>
						</>
					)}
				/>
			</DialogContent>
		</Dialog>
	);
}
