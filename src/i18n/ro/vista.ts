import type { Diccionario } from "../tipos";

// Vista previa: hojas, barra, menú de carta y diálogo de mover.
export const VISTA: Diccionario = {
	"Cartas ordenadas": "Cărți aranjate",
	"De 1 a {n}. Cada página tiene sus casillas seguidas.":
		"De la 1 la {n}. Pozițiile fiecărei pagini sunt consecutive.",
	"Mover a posición": "Mută pe poziția",
	"Mover": "Mută",
	"Posición": "Poziție",
	"Añadir carta": "Adaugă carte",
	"Añadir carta de Scryfall…": "Adaugă carte din Scryfall…",
	"Eliminar las cartas de esta página": "Șterge cărțile de pe această pagină",
	"Página {n} de {total}": "Pagina {n} din {total}",
	"Acercar": "Mărește",
	"Alejar": "Micșorează",
	"Volver al 100 %": "Revino la 100 %",
	"Auto ordenar": "Aranjare automată",
	"Añadir página": "Adaugă pagină",
	"El deck está vacío": "Pachetul este gol",
	"Hueco": "Poziție goală",
	"Info de carta": "Informații carte",
	"Junta las copias, quita los huecos y deja las fijadas en su sitio":
		"Grupează copiile, elimină pozițiile goale și lasă cărțile fixate la locul lor",
	"La vista es aproximada; el resultado exacto es el del PDF.":
		"Previzualizarea este aproximativă; rezultatul exact este cel din PDF.",
	"Ocultar DFC": "Ascunde DFC",
	"Pega una lista a la izquierda y pulsa «Cargar lista», o añade cartas una a una.":
		"Lipește o listă în stânga și apasă „Încarcă lista” sau adaugă cărțile una câte una.",
	"Quita las cartas de doble cara de las hojas y del PDF":
		"Elimină cărțile cu două fețe din foi și din PDF",
	"Se puede deshacer con Ctrl+Z.": "Poți anula cu Ctrl+Z.",
	"Todas": "Toate",
	"Ver reversos": "Arată versourile",
	"Vista {papel}": "Previzualizare {papel}",
	"{n} cartas de doble cara sin dúplex: su otra cara no se imprime (activa «Pares DFC intercalados» o el dúplex).":
		{
			one: "{n} carte cu două fețe fără duplex: cealaltă față nu se tipărește (activează „Perechi DFC intercalate” sau duplexul).",
			few: "{n} cărți cu două fețe fără duplex: cealaltă față nu se tipărește (activează „Perechi DFC intercalate” sau duplexul).",
			other:
				"{n} de cărți cu două fețe fără duplex: cealaltă față nu se tipărește (activează „Perechi DFC intercalate” sau duplexul).",
		},
	"{n} cartas sin reverso: el deck no tiene reverso por defecto.": {
		one: "{n} carte fără verso: pachetul nu are verso implicit.",
		few: "{n} cărți fără verso: pachetul nu are verso implicit.",
		other: "{n} de cărți fără verso: pachetul nu are verso implicit.",
	},
	"{papel} apaisado": "{papel} orizontal",
	"¿Eliminar las cartas de esta página?": "Ștergi cărțile de pe această pagină?",
	"Cambiar reverso": "Schimbă versoul",
	"Exportar imagen": "Exportă imaginea",
	"Frente…": "Față…",
	"reverso": "verso",
	"Reverso…": "Verso…",
	"Imagen guardada: {nombre}": "Imagine salvată: {nombre}",
	"PNG, JPEG, WebP, TIFF o BMP, según la extensión del nombre. A los PPP de la calidad del PDF y sin sangrado.":
		"PNG, JPEG, WebP, TIFF sau BMP, după extensia numelui. La DPI-ul calității PDF-ului și fără sângerare.",
	"Cambiar variante": "Schimbă varianta",
	"Desfijar": "Deblochează",
	"Editar carta": "Editează cartea",
	"Elegir imagen…": "Alege imaginea…",
	"Eliminar las seleccionadas": "Șterge cele selectate",
	"Fijar": "Fixează",
	"Mover a posición…": "Mută pe poziția…",
	"Mover antes": "Mută înainte",
	"Mover después": "Mută după",
	"Reemplazar imagen…": "Înlocuiește imaginea…",
	"Usar el reverso del deck": "Folosește versoul pachetului",
	"Idioma de la imagen": "Limba imaginii",
	"Sin indicar": "Nespecificată",
	"Marcadas como variante de diseño, opcional o banquillo: se imprimen, pero no cuentan para el mazo.":
		"Marcate ca variantă de design, opțională sau sideboard: se tipăresc, dar nu se numără în pachet.",
	"{n} marcadas": {
		one: "{n} marcată",
		few: "{n} marcate",
		other: "{n} marcate",
	},
	"Marca": "Marcaj",
	"Se imprimen igual; no cuentan para el tamaño del mazo, las copias ni las estadísticas.":
		"Se tipăresc la fel; nu se numără în dimensiunea pachetului, limita de copii sau statistici.",
	"Variante de diseño": "Variantă de design",
	"VARIANTE": "VARIANTĂ",
	"Otra versión de una carta que ya está: se escogerá una. De sus copias solo cuenta una para el mazo.":
		"Altă versiune a unei cărți deja prezente: se va alege una. Dintre copiile ei, doar una se numără în pachet.",
	"Opcional": "Opțională",
	"OPCIONAL": "OPȚIONALĂ",
	"Puede ir o no: no cuenta para el mazo.": "Poate intra sau nu: nu se numără în pachet.",
	"Banquillo": "Sideboard",
	"BANQUILLO": "SIDEBOARD",
	"Del banquillo (sideboard): no cuenta para el mazo.": "Din sideboard: nu se numără în pachet.",
};
