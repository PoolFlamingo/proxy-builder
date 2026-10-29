import { useMemo, useState } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
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
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { hacer } from "../acciones";
import { caraVisible, ImagenCarta } from "../imagen-carta";
import { type DialogoMesa, usePantalla } from "../pantalla";

type Pila = "arriba" | "abajo" | "cementerio" | "exilio";

/**
 * **Mirar, escrutar y vigilar** (`capturas/pablo/15`): las cartas que se
 * miran (las que Rust ha enseñado al jugador) se reparten entre arriba (en
 * orden), abajo, cementerio y exilio, y se confirma con `Ordenar`. Cerrar sin
 * confirmar las deja donde estaban (y se olvidan).
 */
export function MirarDialog({ d }: { d: Extract<DialogoMesa, { tipo: "mirar" }> }) {
	const { t } = useLang();
	const lib = useMesa((s) => s.zonas[`${d.de}:biblioteca`]);
	const cartas = useMesa((s) => s.cartas);
	// Las miradas, en su orden en la biblioteca (de arriba abajo).
	const miradas = useMemo(
		() =>
			(lib?.visibles ?? [])
				.filter(([, id]) => cartas[id]?.mirada)
				.sort(([a], [b]) => a - b)
				.map(([, id]) => id),
		[lib, cartas],
	);
	const [reparto, setReparto] = useState<Record<IdCarta, Pila>>({});
	const [orden, setOrden] = useState<IdCarta[] | null>(null);
	const lista = orden ?? (d.desde === "abajo" ? [...miradas].reverse() : miradas);
	const donde = (id: IdCarta): Pila => reparto[id] ?? (d.desde === "abajo" ? "abajo" : "arriba");
	const cerrar = () => {
		usePantalla.getState().abrir(null);
		void hacer({ que: "dejarDeMirar", de: d.de });
	};
	const confirmar = () => {
		const de = (p: Pila) => lista.filter((id) => donde(id) === p);
		usePantalla.getState().abrir(null);
		void hacer({
			que: "ordenar",
			de: d.de,
			arriba: de("arriba"),
			abajo: de("abajo"),
			cementerio: de("cementerio"),
			exilio: de("exilio"),
		});
	};
	const subir = (i: number, delta: number) => {
		const v = [...lista];
		const j = i + delta;
		if (j < 0 || j >= v.length) return;
		[v[i], v[j]] = [v[j] as IdCarta, v[i] as IdCarta];
		setOrden(v);
	};
	const opciones: { pila: Pila; icono: React.ReactNode; nombre: string }[] = [
		{
			pila: "arriba",
			icono: <Icono icon="lucide:arrow-up" className="size-3.5" />,
			nombre: t("Arriba"),
		},
		{
			pila: "abajo",
			icono: <Icono icon="lucide:arrow-down" className="size-3.5" />,
			nombre: t("Abajo"),
		},
		{
			pila: "cementerio",
			icono: <Icono icon="game-icons:tombstone" className="size-3.5" />,
			nombre: t("Cementerio"),
		},
		{
			pila: "exilio",
			icono: <Icono icon="lucide:trash-2" className="size-3.5" />,
			nombre: t("Exilio"),
		},
	];
	const titulo =
		d.modo === "escrutar" ? t("Escrutar") : d.modo === "vigilar" ? t("Vigilar") : t("Mirar");
	const permitidas =
		d.modo === "escrutar"
			? opciones.filter((o) => o.pila === "arriba" || o.pila === "abajo")
			: d.modo === "vigilar"
				? opciones.filter((o) => o.pila === "arriba" || o.pila === "cementerio")
				: opciones;
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-3xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{titulo}</DialogTitle>
					<DialogDescription>
						{t("Las que vayan arriba quedan en este orden (la primera, la de arriba del todo).")}
					</DialogDescription>
				</DialogHeader>
				<ScrollArea viewportClassName="max-h-[60vh]">
					<div className="flex flex-wrap gap-2.5 p-1">
						{lista.map((id, i) => {
							const c = cartas[id];
							if (!c) return null;
							return (
								<div key={id} className="flex w-36 flex-col gap-1">
									<div
										className={cn(
											"h-[200px] overflow-hidden rounded-[5%]",
											donde(id) !== "arriba" && "opacity-60",
										)}
										onPointerEnter={() => useMesa.getState().setSobre(id)}
										onPointerLeave={() => useMesa.getState().setSobre(null)}
									>
										<ImagenCarta
											imagen={caraVisible(c, true)}
											ancho={244}
											nombre={c.origen?.nombre}
										/>
									</div>
									<div className="flex justify-center gap-1">
										<Button
											size="icon-sm"
											variant="ghost"
											onClick={() => subir(i, -1)}
											aria-label={t("Antes")}
										>
											<Icono icon="lucide:chevron-up" className="size-3.5" />
										</Button>
										<Button
											size="icon-sm"
											variant="ghost"
											onClick={() => subir(i, 1)}
											aria-label={t("Después")}
										>
											<Icono icon="lucide:chevron-down" className="size-3.5" />
										</Button>
									</div>
									<div className="grid grid-cols-2 gap-1">
										{permitidas.map((o) => (
											<Button
												key={o.pila}
												size="sm"
												variant={donde(id) === o.pila ? "default" : "outline"}
												className="h-6 px-1 text-[10px]"
												onClick={() => setReparto({ ...reparto, [id]: o.pila })}
											>
												{o.icono}
												{o.nombre}
											</Button>
										))}
									</div>
								</div>
							);
						})}
						{lista.length === 0 && (
							<p className="p-4 text-xs text-muted-foreground">
								{t("No queda ninguna carta ahí.")}
							</p>
						)}
					</div>
				</ScrollArea>
				<DialogFooter>
					<Button variant="outline" onClick={cerrar}>
						{t("Dejarlas como están")}
					</Button>
					<Button onClick={confirmar} disabled={lista.length === 0}>
						{t("Confirmar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
