import { Swords } from "lucide-react";
import { toast } from "sonner";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";

/** Cuánto se queda el aviso a la vista, en ms (hay que tener tiempo de decidir). */
const MS_AVISO = 20_000;

/**
 * **El aviso del bono de un equipo o un aura** al adjuntarlo: una tarjeta con
 * el bono en grande, qué equipo es y a qué criatura, y dos botones. Es más
 * llamativo que un aviso corriente a propósito: es una decisión de quien
 * juega (el motor no suma el bono solo) y se pasaba por alto.
 */
export function avisarBono(o: {
	equipo: string;
	criatura: string;
	/** Ya con signo: «+2/+2». */
	bono: string;
	alSumar: () => void;
}) {
	toast.custom(
		(id) => (
			<AvisoBono
				{...o}
				alSumar={() => {
					toast.dismiss(id);
					o.alSumar();
				}}
				alCerrar={() => toast.dismiss(id)}
			/>
		),
		{ duration: MS_AVISO },
	);
}

function AvisoBono({
	equipo,
	criatura,
	bono,
	alSumar,
	alCerrar,
}: {
	equipo: string;
	criatura: string;
	bono: string;
	alSumar: () => void;
	alCerrar: () => void;
}) {
	const { t } = useLang();
	return (
		<div className="w-[340px] rounded-xl border-2 border-primary bg-popover p-3 text-popover-foreground shadow-2xl ring-4 ring-primary/25">
			<div className="mb-2 flex items-center gap-1.5 text-[10px] font-semibold tracking-wide text-primary uppercase">
				<Swords className="size-3.5" />
				{t("Bono al adjuntar")}
			</div>
			<div className="flex items-center gap-3">
				<span className="animate-pulse rounded-lg bg-primary px-3 py-1.5 text-2xl font-extrabold text-primary-foreground tabular-nums [animation-iteration-count:4]">
					{bono}
				</span>
				<div className="min-w-0 text-xs">
					<div className="truncate font-semibold">{equipo}</div>
					<div className="truncate text-muted-foreground">→ {criatura}</div>
				</div>
			</div>
			<div className="mt-3 flex gap-2">
				<Button className="flex-1" onClick={alSumar}>
					{t("Sumar {bono}", { bono })}
				</Button>
				<Button variant="outline" onClick={alCerrar}>
					{t("Ahora no")}
				</Button>
			</div>
		</div>
	);
}
