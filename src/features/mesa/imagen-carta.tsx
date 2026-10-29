import { useState } from "react";
import type { RefImagen } from "@/bindings/RefImagen";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { Icono } from "@/components/icono";
import { type AnchoMesa, urlMesa } from "@/lib/image-url";
import { cn } from "@/lib/utils";

/** Lo que se espera para volver a pedir una imagen que no llegó, en ms. */
const REINTENTO_MS = 4000;

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
			<Icono icon="lucide:layers" className="size-1/3 text-primary/70" />
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
	// La URL que no se pudo cargar: en su lugar, el reverso (nunca un hueco).
	const [fallida, setFallida] = useState<string | null>(null);
	if (!url || fallida === url) {
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
		// Sin `loading="lazy"`: en WebKitGTK retrasa las de cajas que se mueven. El
		// fondo oscuro guarda la forma de la carta mientras llega.
		<img
			src={url}
			alt={nombre ?? ""}
			draggable={false}
			decoding="async"
			onError={() => {
				setFallida(url);
				// En red puede llegar más tarde (el anfitrión aún no la tenía): se reintenta.
				window.setTimeout(() => setFallida((f) => (f === url ? null : f)), REINTENTO_MS);
			}}
			className={cn("size-full rounded-[5%] bg-slate-900 object-cover select-none", className)}
		/>
	);
}
