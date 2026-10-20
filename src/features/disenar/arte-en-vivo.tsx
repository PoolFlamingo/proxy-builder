import { useEffect, useState } from "react";
import type { ArtFraming } from "@/bindings/ArtFraming";
import type { CapaInfo } from "@/bindings/CapaInfo";
import { cn } from "@/lib/utils";
import { cubreElHueco, geometriaArte } from "./encuadre";

/**
 * La carta montada en la pantalla mientras se mueve la ilustración: fondo
 * negro, la ilustración en su hueco (movida con CSS, al instante) y encima
 * la carta sin ilustración que pinta Rust una sola vez (`design_overlay`).
 * Siempre montada cuando hay capa, para que las dos imágenes estén cargadas
 * antes de hacer falta; `visible` decide si se ve.
 */
export function ArteEnVivo({
	capa,
	urlCapa,
	urlArte,
	framing,
	visible,
	onLista,
}: {
	capa: CapaInfo;
	urlCapa: string;
	urlArte: string;
	framing: ArtFraming;
	visible: boolean;
	/** Las dos imágenes cargadas (o ya no). */
	onLista: (lista: boolean) => void;
}) {
	// Medida natural de la ilustración y qué URL está cargada de cada una.
	const [arte, setArte] = useState<{ url: string; w: number; h: number } | null>(null);
	const [capaCargada, setCapaCargada] = useState<string | null>(null);
	const lista = arte?.url === urlArte && capaCargada === urlCapa;
	useEffect(() => onLista(lista), [lista, onLista]);

	const [x, y, w, h] = capa.arte;
	const extension = capa.extension;
	// El hueco en milímetros de la carta de 63 × 88: la proporción es lo que cuenta.
	const g = arte ? geometriaArte(framing, arte.w, arte.h, w * 63, h * 88) : null;
	return (
		<div
			aria-hidden
			className={cn(
				"pointer-events-none absolute inset-0 overflow-hidden bg-black shadow-lg",
				!(visible && lista) && "invisible",
			)}
			style={{ borderRadius: `${capa.radio[0] * 100}% / ${capa.radio[1] * 100}%` }}
		>
			{/* El recorte llega hasta el final de la extensión del difuminado;
			    dentro, el hueco de siempre (la cuenta del encuadre es la suya). */}
			<div
				className="absolute overflow-hidden"
				style={{
					left: `${x * 100}%`,
					top: `${y * 100}%`,
					width: `${w * 100}%`,
					height: `${(h + extension) * 100}%`,
				}}
			>
				{/* Si la imagen no cubre el hueco (zoom por debajo de 1), lo que
				    falta, del color de su borde (Rust alarga los bordes y los
				    desenfoca; mientras se mueve, basta con su color). */}
				{!cubreElHueco(framing) && (
					<div
						className="absolute inset-0"
						style={{ backgroundColor: `rgb(${capa.fondo.join(",")})` }}
					/>
				)}
				<div
					className="absolute top-0 left-0 w-full"
					style={{ height: `${(h / (h + extension)) * 100}%` }}
				>
					<img
						src={urlArte}
						alt=""
						draggable={false}
						onLoad={(e) => {
							const i = e.currentTarget;
							setArte({ url: urlArte, w: i.naturalWidth, h: i.naturalHeight });
						}}
						className="absolute top-0 left-0 max-w-none will-change-transform"
						style={
							g
								? {
										width: `${g.ancho}%`,
										height: `${g.alto}%`,
										transform: `translate(${g.tx}%, ${g.ty}%) rotate(${g.giro}deg)`,
									}
								: { visibility: "hidden" }
						}
					/>
				</div>
			</div>
			<img
				src={urlCapa}
				alt=""
				draggable={false}
				onLoad={() => setCapaCargada(urlCapa)}
				className="absolute inset-0 size-full"
			/>
		</div>
	);
}
