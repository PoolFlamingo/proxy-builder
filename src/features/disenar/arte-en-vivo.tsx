import { useEffect, useState } from "react";
import type { ArtFraming } from "@/bindings/ArtFraming";
import type { CapaInfo } from "@/bindings/CapaInfo";
import { cn } from "@/lib/utils";
import { CARTA_MM, cubreElHueco, deLado, geometriaArte, huecoSinGirar } from "./encuadre";

/**
 * La carta montada en la pantalla mientras se mueve la ilustración: fondo
 * negro, la ilustración en su hueco (movida con CSS, al instante) y encima
 * la carta sin ilustración que pinta Rust una sola vez (`design_overlay`).
 * Siempre montada cuando hay capa, para que las dos imágenes estén cargadas
 * antes de hacer falta; `visible` decide si se ve. En las piezas giradas
 * (la mitad de abajo de una Secuela, las partidas) la ilustración va en una
 * caja sin girar, del tamaño del hueco tumbado, que se gira entera.
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
	const giro = capa.giro ?? 0;
	// Girada, la extensión del difuminado no va hacia abajo (Rust no la da).
	const extension = giro ? 0 : capa.extension;
	// El hueco en milímetros de la carta de 63 × 88 (la proporción es lo que
	// cuenta), tal como lo ve la ilustración.
	const sin = huecoSinGirar(capa.arte, giro);
	const g = arte ? geometriaArte(framing, arte.w, arte.h, sin.w, sin.h) : null;
	// La caja sin girar, en % del hueco, centrada en él.
	const caja = deLado(giro)
		? {
				w: ((h * CARTA_MM.h) / (w * CARTA_MM.w)) * 100,
				h: ((w * CARTA_MM.w) / (h * CARTA_MM.h)) * 100,
			}
		: { w: 100, h: 100 };
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
					className="absolute"
					style={{
						width: `${caja.w}%`,
						height: `${(caja.h * h) / (h + extension)}%`,
						left: `${(100 - caja.w) / 2}%`,
						top: `${((100 - caja.h) / 2) * (h / (h + extension))}%`,
						transform: giro ? `rotate(${giro}deg)` : undefined,
					}}
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
