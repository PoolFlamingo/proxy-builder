import { useEffect, useState } from "react";
import type { OrigenCarta } from "@/bindings/OrigenCarta";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { ImagenCarta } from "../imagen-carta";

/**
 * **La carta del fantasma**, por su nombre: la busca Rust en Scryfall
 * (`mesa_sparring_carta`) y se recuerda en esta sesión. Sin red, el nombre y
 * el coste sobre el reverso.
 */

const pedidas = new Map<string, Promise<OrigenCarta | null>>();

function pedir(nombre: string): Promise<OrigenCarta | null> {
	let p = pedidas.get(nombre);
	if (!p) {
		p = call("mesa_sparring_carta", { nombre }).catch(() => {
			pedidas.delete(nombre);
			return null;
		});
		pedidas.set(nombre, p);
	}
	return p;
}

export function CartaFantasma({
	nombre,
	ancho,
	className,
}: {
	nombre: string;
	ancho: 244 | 488;
	className?: string;
}) {
	const [origen, setOrigen] = useState<OrigenCarta | null>(null);
	useEffect(() => {
		let vivo = true;
		setOrigen(null);
		void pedir(nombre).then((o) => vivo && setOrigen(o));
		return () => {
			vivo = false;
		};
	}, [nombre]);
	return (
		<div className={cn("aspect-[63/88]", className)}>
			<ImagenCarta
				imagen={origen?.caras[0] ?? { tipo: "generica" }}
				ancho={ancho}
				nombre={nombre}
			/>
		</div>
	);
}
