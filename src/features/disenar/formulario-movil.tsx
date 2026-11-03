import type { CardDesign } from "@/bindings/CardDesign";
import type { Collection } from "@/bindings/Collection";
import type { PackInfo } from "@/bindings/PackInfo";
import type { Resumen } from "@/bindings/Resumen";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
	FilasEstilo,
	MarcoEditable,
	PestanaArte,
	PestanaPie,
	PestanaTextos,
	porCaras,
	SelectorParte,
} from "./formulario";

/**
 * El formulario del móvil: lo básico del editor en cuatro pestañas bajo la
 * carta (decisión de Pablo, 25/10/2026). Son las mismas piezas que el
 * formulario de siempre, repartidas de otra manera:
 *
 * - **Textos**: nombre, tipo, ambientación y reglas, y el pie.
 * - **Ilustración**: elegirla y moverla (con los dedos, sobre la carta).
 * - **Marco**: el estilo y su variante, el pack y la corona.
 * - **Colores**: el color del marco y la caja de reglas.
 *
 * No entra (se hace en una tableta o en el ordenador): ajustes finos, letras,
 * packs y símbolos, colecciones, traducción sugerida, diseñar en lote,
 * renumerar y exportar imágenes. Cambiar el diseño es lo mismo que en el
 * formulario de siempre, así que la carta sale con la misma huella.
 */
export function FormularioMovil({
	design,
	cambiar,
	cara,
	setCara,
	colecciones,
	packs,
	resumen,
}: {
	design: CardDesign;
	cambiar: (f: (d: CardDesign) => CardDesign) => void;
	cara: number;
	setCara: (c: number) => void;
	colecciones: Collection[];
	packs: PackInfo[];
	resumen: Resumen | null;
}) {
	const { t } = useLang();
	const { vista, cambiarVista, porCara, contenido } = porCaras({
		design,
		cambiar,
		cara,
		setCara,
		resumen,
	});
	const parte = <SelectorParte design={design} cara={cara} setCara={setCara} resumen={resumen} />;
	return (
		<Tabs
			defaultValue="textos"
			className="flex h-full min-h-0 flex-col gap-0"
			data-tutorial="disenar-formulario"
		>
			<div className="shrink-0 border-b px-2 py-1.5">
				<TabsList className="w-full [[data-tactil]_&]:h-11">
					<TabsTrigger value="textos" className="px-1">
						{t("Textos")}
					</TabsTrigger>
					<TabsTrigger value="arte" className="px-1">
						{t("Ilustración")}
					</TabsTrigger>
					<TabsTrigger value="marco" className="px-1">
						{t("Marco")}
					</TabsTrigger>
					<TabsTrigger value="colores" className="px-1">
						{t("Colores")}
					</TabsTrigger>
				</TabsList>
			</div>
			<ScrollArea className="min-h-0 flex-1">
				<div className="p-2.5">
					<TabsContent value="textos" className="flex flex-col gap-3">
						{parte}
						<PestanaTextos design={design} cambiar={cambiar} cara={cara} traducir={false} />
						<div className="h-px bg-border" />
						<PestanaPie design={design} cambiar={cambiar} colecciones={colecciones} />
					</TabsContent>
					<TabsContent value="arte" className="flex flex-col gap-3">
						{parte}
						{resumen?.artePropio === false ? (
							<div className="flex flex-col items-start gap-2 text-xs text-muted-foreground">
								<p>
									{t(
										"Esta parte no lleva ilustración propia: enseña la de la parte 1, como las cartas impresas.",
									)}
								</p>
								<Button size="sm" variant="outline" onClick={() => setCara(0)}>
									{t("Ir a la parte 1")}
								</Button>
							</div>
						) : (
							<PestanaArte design={design} cambiar={cambiar} cara={cara} />
						)}
					</TabsContent>
					<TabsContent value="marco" className="flex flex-col gap-3">
						{porCara}
						{contenido(
							<>
								<FilasEstilo
									design={vista}
									cambiar={cambiarVista}
									colecciones={colecciones}
									packs={packs}
								/>
								<MarcoEditable
									design={vista}
									cambiar={cambiarVista}
									colecciones={colecciones}
									packs={packs}
									parte="marco"
								/>
							</>,
						)}
					</TabsContent>
					<TabsContent value="colores" className="flex flex-col gap-3">
						{porCara}
						{contenido(
							<MarcoEditable
								design={vista}
								cambiar={cambiarVista}
								colecciones={colecciones}
								packs={packs}
								parte="colores"
							/>,
						)}
					</TabsContent>
				</div>
			</ScrollArea>
		</Tabs>
	);
}
