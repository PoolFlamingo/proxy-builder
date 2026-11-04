import { Bandera } from "@/components/emoji";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { IDIOMAS, type Idioma, type Lang } from "@/lib/idiomas";
import { cn } from "@/lib/utils";

/**
 * Un desplegable de idiomas con su bandera y su nombre propio. Lo usan el
 * idioma de la interfaz (Ajustes) y el de las cartas diseñadas.
 */
export function SelectorIdioma({
	valor,
	onCambio,
	idiomas = IDIOMAS,
	ancho = "w-44",
	ariaLabel,
	soloBandera = false,
}: {
	valor: Lang;
	onCambio: (l: Lang) => void;
	idiomas?: readonly Idioma[];
	ancho?: string;
	ariaLabel?: string;
	/** Cerrado, solo la bandera (donde no cabe el nombre: junto a la carta en el móvil). */
	soloBandera?: boolean;
}) {
	const elegido = idiomas.find((i) => i.codigo === valor);
	return (
		<Select value={valor} onValueChange={(v) => onCambio(v as Lang)}>
			<SelectTrigger size="sm" className={cn("h-7 text-xs", ancho)} aria-label={ariaLabel}>
				{soloBandera && elegido ? (
					<SelectValue>
						<Bandera pais={elegido.pais} alt="" />
					</SelectValue>
				) : (
					<SelectValue />
				)}
			</SelectTrigger>
			<SelectContent>
				{idiomas.map((i) => (
					<SelectItem key={i.codigo} value={i.codigo} className="text-xs" lang={i.locale}>
						<Bandera pais={i.pais} alt="" />
						{i.nativo}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}
