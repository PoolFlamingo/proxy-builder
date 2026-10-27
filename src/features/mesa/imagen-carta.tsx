import { Layers } from "lucide-react";
import type { RefImagen } from "@/bindings/RefImagen";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { type AnchoMesa, urlMesa } from "@/lib/image-url";
import { cn } from "@/lib/utils";

/** La cara que se ve de una carta: la suya, o el dorso si está boca abajo o no se conoce. */
export function caraVisible(c: VistaCarta, verBocaAbajo = false): RefImagen {
	if (!c.origen || (c.bocaAbajo && !verBocaAbajo)) return c.dorso;
	return c.origen.caras[c.cara] ?? c.origen.caras[0] ?? c.dorso;
}

/** El reverso de la app, pintado (no es una imagen). */
export function DorsoGenerico({ className }: { className?: string }) {
	return (
		<div
			className={cn(
				"flex size-full items-center justify-center rounded-[5%] border border-white/10 bg-gradient-to-br from-slate-800 via-slate-900 to-black",
				className,
			)}
		>
			<Layers className="size-1/3 text-primary/70" />
		</div>
	);
}

/** La imagen de una cara (o el reverso genérico). */
export function ImagenCarta({
	imagen,
	ancho,
	nombre,
	className,
}: {
	imagen: RefImagen;
	ancho: AnchoMesa;
	nombre?: string;
	className?: string;
}) {
	const url = urlMesa(imagen, ancho);
	if (!url) {
		return (
			<div className={cn("relative size-full", className)}>
				<DorsoGenerico />
				{nombre && (
					<span className="absolute inset-x-1 top-1 line-clamp-2 rounded bg-black/60 px-1 text-center text-[9px] text-white">
						{nombre}
					</span>
				)}
			</div>
		);
	}
	return (
		<img
			src={url}
			alt={nombre ?? ""}
			draggable={false}
			loading="lazy"
			decoding="async"
			className={cn("size-full rounded-[5%] object-cover select-none", className)}
		/>
	);
}
