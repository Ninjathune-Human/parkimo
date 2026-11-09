# Trame

![Trame, étude de capacité de stationnement](docs/banniere.png)

**[Ouvrir l'application](https://ninjathune-human.github.io/parkimo/)**

Étude de capacité de stationnement en temps réel, dans l'esprit de TestFit, adaptée aux règles françaises.
Moteur en Rust compilé en WebAssembly, interface en un seul fichier `index.html` (PC, iPad, iPhone, tactile et stylet).

## Ce que fait le moteur

Un pipeline unique, rejoué à chaque mouvement de poignée :

1. **Parcelle et reculs par limite** : chaque arête a son propre recul (`setbacks[i]`), calculé comme ensemble des points à moins de *s<sub>i</sub>* de l'arête.
2. **Obstacles** : servitudes, bâtiments (avec trottoir), espaces verts imposés sont soustraits. Résultat : la zone stationnable **B**.
3. **Places en bordure** : on longe chaque bord de B, droit ou courbe, en posant des places perpendiculaires au bord local. Les angles saillants sont dégagés par une longueur calculée (`d·cot(β/2)` et zone de manœuvre pour les angles aigus).
4. **Anneau de circulation** : B érodée de la profondeur de place. Toute chaussée plus étroite que l'allée (pointes en impasse dans les angles aigus, effilements) est retirée par une ouverture morphologique, sauf l'aire de manœuvre devant les places ; les angles saillants de la chaussée sont arrondis.
5. **Rangées doubles au cœur** : B érodée de (place + allée), remplie de rangées dos à dos. L'optimiseur teste l'orientation (directions des limites et balayage tous les 3°) et le décalage de la trame (pas de 0,25 m), puis affine autour des meilleures solutions : environ 4 000 à 5 000 variantes par calcul.
6. **Découpe exacte** : îlots plantés répartis régulièrement et alignés sur les deux demi-rangées, îlots d'extrémité, allées transversales là où le concepteur a tracé une traverse (plus aucune coupure automatique).
   Tout espace résiduel de la zone stationnable (angles, têtes de rangée en biais, interstices entre places) devient un espace planté, réuni aux îlots d'extrémité et de bordure : même teinte, calque Îlots plantés, calque AutoCAD `PRJ_ILOTS`. Pour faciliter la giration, tous les angles saillants tournés vers la chaussée sont arrondis au rayon de 1 m (moins si les côtés sont courts), la chaussée reprenant l'arrondi ; les filets et décrochés de quelques centimètres laissés par les opérations booléennes (rangées légèrement en biais) sont résorbés avant ; un angle contre une place reste vif, si bien que les bandes plantées entre deux places sont rectangulaires.
7. **Accès et connectivité** : rasterisation de la voirie à 0,5 m et propagation depuis les accès ; toute place sans accès est retirée et signalée.
   Entrée de parking : de chaque côté de l'accès, là où il débouche sur l'allée, un arrondi de 3 m de rayon facilite la giration ; les places en bordure sont repoussées au-delà de ce rayon.
8. **PMR et IRVE** : groupes de 4 places remplacés par 3 places PMR de 3,30 m au plus près de l'entrée du bâtiment ; places IRVE pré-équipées (gaines, 20 %) marquées, dont au moins une PMR, et parmi elles les places équipées d'une borne (1 par tranche de 20 places), la PMR d'abord, repérées par un point au fond de la place.
9. **Circulation et signalisation horizontale** : voir ci-dessous.

Temps de calcul mesurés (WebAssembly, Chromium) : 2 à 12 ms en mode direct pendant un glisser, 12 à 40 ms pour l'optimisation complète au relâchement. Le retrait des pointes et l'analyse de circulation ne sont faits qu'au relâchement.

## Circulation et signalisation horizontale

Le moteur reconstitue le fonctionnement du parking du point de vue de l'automobiliste, puis peint les flèches au sol (`src/circulation.rs`).

1. **Réseau des voies** : la chaussée (anneau, allées, allées transversales, accès) est tramée à 0,5 m ; une transformée en distance exacte donne la largeur locale, un amincissement donne les axes. Les axes forment un graphe de carrefours, d'impasses et d'accès ; les éperons d'angle sont élagués.
2. **Logique de circulation** : on entre par un accès, on parcourt les allées pour trouver une place, on ressort par un accès. Une voie d'au moins 5,00 m est à **double sens** (NF P 91-120 ; 5,50 m en NF P 91-100). Une impasse, ou un passage obligé comme un accès unique, reste à double sens même étroit : il faut pouvoir en ressortir (signalé dans le bilan). Les autres voies étroites passent à **sens unique**, orientées pour que chaque voie soit atteignable depuis la rue et que l'on puisse toujours regagner la rue (orientation fortement connexe), en tournant de préférence dans le sens anti-horaire, comme sur un giratoire. Avec plusieurs accès étroits, certains deviennent des entrées et d'autres des sorties.
3. **Flèches au sol** : flèche **double** sur une voie à double sens (une flèche par file, circulation à droite), flèche **simple** au centre d'une voie à sens unique. Une seule flèche (ou une seule paire en double sens) par artère, au centre de l'arête du squelette, décalée au besoin sur une partie droite ; une flèche n'est posée que sur une partie droite, entièrement dans la chaussée, sans chevaucher une autre.

Avec les allées de 6,00 m par défaut, tout le parking est à double sens. Régler la largeur d'allée sous 5,00 m dans Paramètres produit un plan à sens unique.

**Squelette des voies** (interrupteur du menu Modifier) : les axes du réseau s'affichent sur le plan, en double trait pour une voie à double sens, en trait simple fléché pour une voie à sens unique. Au milieu de chaque voie (d'un carrefour au suivant), une pastille porte deux boutons : le **sens de parcours**, que l'on inverse d'un toucher sur une voie à sens unique, et le **nombre de voies** (2 ⇄ 1). Quand des voies parallèles sont proches, la pastille glisse un peu le long de la voie ; trop courte à l'écran, une voie n'en a pas (zoomer). Le choix est enregistré dans le projet (annuler, rétablir, export) par un repère que le moteur recale à chaque calcul sur le milieu de la voie, avec son sens de parcours. Les sens uniques imposés sont orientés avant ceux des voies étroites, du plus ancien au plus récent, avec la même règle de connexité : un sens unique, ou un sens de parcours, qui couperait la circulation (impasse, passage obligé) est refusé et le plan reste inchangé. Une voie étroite sans consigne prolonge de préférence le sens d'une voie alignée.

**Estimation des travaux** (menu Estimation) : montant global au ratio, d'après le nombre de places du plan et un barème de parkings de surface de la métropole niçoise (€ HT par place, travaux seuls puis avec études et maîtrise d'œuvre) : 20 à 50 places 2 300 à 3 800 € (2 700 à 4 400 €) ; 50 à 150 : 2 000 à 3 200 € (2 350 à 3 700 €) ; 150 à 300 : 1 800 à 2 800 € (2 100 à 3 250 €) ; 300 à 600 : 1 600 à 2 500 € (1 850 à 2 900 €) ; 600 à 1 000 : 1 500 à 2 300 € (1 750 à 2 650 €). La tranche est celle qui contient le nombre de places (à 50 places, la tranche inférieure) ; hors barème, la tranche extrême s'applique. Les ratios sont modifiables dans le menu (le barème est enregistré dans le navigateur, hors projet ; « Barème d'origine » le rétablit). Le montant se met à jour à chaque calcul. Estimation d'avant-projet, hors foncier et aléas de site.

**Traverses** (menu Modifier, affichées avec le squelette des voies) : un segment tracé par le concepteur coupe les rangées en allées transversales à la largeur de l'allée. On déplace ses deux extrémités (poignées rondes), et la croix, au milieu, la supprime. Les créneaux retirés suivent la traverse rangée par rangée ; une traverse parallèle aux rangées n'en coupe aucune. Elles sont enregistrées dans le projet (annuler, rétablir). Le réglage « Rangée continue maximale » n'existe plus.

**Squelette des noues** (second interrupteur du menu Modifier) : une barre symbolique s'affiche dans l'axe de chaque noue, avec une pastille en son milieu. Un toucher sur la pastille (croix) **supprime** la noue : son figuré disparaît, la rangée se referme (les deux demi-rangées se touchent), les rangées au-dessus se rapprochent d'autant et de nouvelles places prennent la place libérée, pour garder un maximum de places. La suppression vaut pour toute la rangée (tous ses tronçons). La noue n'est plus exportée en DXF. La barre devient alors **rouge** et reste visible dans le squelette ; la pastille (flèche de retour) la **rétablit**. Le choix est enregistré dans le projet (annuler, rétablir) par un repère (`noue_off`, milieu de l'axe) que le moteur recale à chaque calcul sur la noue qu'il touche (`noues_off` en sortie).

**Largeur voie en sens unique** (5,00 m par défaut : places à 90°, NF P 91-100) : une voie passée à sens unique prend cette largeur.
- Allée entre deux rangées : elle se resserre et les rangées voisines s'en rapprochent chacune de la moitié de l'écart ; la trame (orientation, décalage) reste celle du plan optimisé, puis glisse d'au plus 2 m pour tirer parti de la place libérée.
- Côté droit de l'anneau perpendiculaire aux rangées : le cœur avance, les rangées s'allongent.
- Côté droit de l'anneau parallèle aux rangées, en bord de trame : la trame se cale pour que la rangée la plus proche affleure la voie resserrée.
- Tronçon oblique ou courbe, allée transversale, raccord : la voie passe à sens unique sans changer de largeur.

Calque **Sens de circulation** dans la couche Projet ; à l'export, bloc `SIG_FLECHE` sur le calque `PRJ_SIGNALISATION`.

## Interface

- **Barre d'outils** : **Nouveau** (projet vierge, Carte IGN, Tracer une parcelle, retour à « Mon site »), puis **Paramètres**, **Ajouter**, **Modifier** (squelettes des voies et des noues, orientation des rangées, traverses), **Habillage**, **Estimation** et **Exporter**, et enfin annuler / rétablir.
- **Colonne de gauche** : le bilan en haut, les couches en bas ; les couches cèdent la hauteur qui manque et défilent. Sur téléphone, les deux panneaux sont repliés en haut et un seul s'ouvre à la fois.
- **Limites de la parcelle** : au survol d'un bord (ou en le touchant sur tablette), deux boutons apparaissent en son milieu, alignés sur le segment : **+** ajoute un sommet, **arc** arrondit le segment ou le redresse. Un segment arrondi est un vrai arc de cercle (flèche mémorisée en fraction de la corde) : on règle la courbure en glissant la poignée située au-delà de son sommet, et l'arc suit le déplacement des sommets. Seul le moteur et l'export DXF reçoivent un contour discrétisé (un côté tous les 4° environ). Ajouter un sommet sur un arc le coupe en deux arcs du même cercle. Les mêmes boutons, la même poignée (visible tant que l'objet est sélectionné ; un clic dans le vide ou Échap désélectionne et masque poignées et boutons) et les mêmes arcs valent pour les segments des servitudes, des espaces verts imposés et des bassins (la flèche est portée par le sommet : `[x, y, k]`). Le long d'une courbe, les places périphériques sont des trapèzes : un seul trait, porté par la normale locale de la limite, sépare deux places voisines, sans interstice ni chevauchement. Quand la limite est bombée (les places se resserrent vers l'allée), le fond s'élargit pour que l'entrée de chaque place garde au moins la largeur demandée.
- **Bassin de rétention** (menu Ajouter ; traits fins et peu contrastés, comme les noues) : zone verte en creux, sans stationnement, éditable comme un espace vert (sommets, arcs, déplacement). Il se représente en topographie : haut de talus (trait fort, le contour du polygone), pied de talus (trait fin, fond plus sombre) à 3 × la hauteur du haut de talus (talus 3 pour 1, réduit si le bassin est étroit) et barbules du haut vers le bas, alternativement longues et courtes. La hauteur se règle dans Paramètres (« Hauteur bassin de rétention », 1,50 m par défaut). Une bande de verdure d'accès et d'entretien de 2 m (réglable, « Verdure autour du bassin ») entoure le haut de talus : aucune place n'y est tracée. Export DXF : calques `PRJ_BASSIN_TALUS_HAUT` et `PRJ_BASSIN_TALUS_BAS` (sans barbules).
- **Entrée des bâtiments** : chaque bâtiment porte une entrée (barre sur son pourtour, largeur réglée dans Paramètres, cheminement à travers le trottoir). Devant l'entrée, un couloir libre de la largeur de l'entrée est réservé : ni place de stationnement ni espace vert n'y est tracé (profondeur : trottoir, plus la profondeur de place si le périmètre est activé). Un avertissement s'affiche si une servitude ou un espace vert posé par l'utilisateur l'obstrue. Le reste de place le long de la limite est réparti à égalité de part et d'autre du couloir (même jeu des deux côtés, sans perdre de place), et les espaces verts qui le bordent ne reçoivent jamais d'arrondi de giration. Glissez sa poignée le long du contour pour la placer ; par défaut, elle est sur l'arête la plus proche du premier accès véhicules. Elle suit le bâtiment qu'on déplace, tourne ou redimensionne, et sert de cible aux places PMR.
- **Bouton de cadrage et échelle** en haut à droite (en bas à droite sur téléphone et sur la carte).

## Fond de carte IGN

Bouton **Carte** : recherche d'un lieu (adresse, commune, lieu-dit, référence cadastrale comme `06088000KY0116` ou `06088 KY 116`, coordonnées GPS), puis **Emprise** pour poser un rectangle ajustable et **Importer**.

- La photo aérienne (ORTHOIMAGERY.ORTHOPHOTOS, jusqu'au niveau 19, environ 0,2 m par pixel) est assemblée puis **découpée au rectangle** : seule cette image est conservée.
- Les données cadastrales du Parcellaire Express (WFS, pagination par 1 000 objets) sont converties dans le repère local du plan et forment des **couches**. Chaque élément affichable ou masquable d'une couche est un **calque**, avec son interrupteur : parcelles, numéros, feuilles et sections, limites communales.
- **Occupation du sol**, calculée en Rust dans l'emprise : le bâti (cadastre), la voirie (domaine non cadastré : l'emprise moins les parcelles), et tout le reste en **végétation**, représentée par une trame verte. Ce sont de vrais polygones (avec trous, par exemple les cours intérieures), au même titre que le bâti.
- Toucher une parcelle puis **Étudier** en fait la parcelle d'étude ; **Ajouter à l'étude** la réunit à la précédente (union calculée en Rust).
- Le fond est enregistré dans le navigateur (IndexedDB) et revient au rechargement.

Services utilisés (Géoplateforme, sans clé) : `data.geopf.fr/wmts`, `data.geopf.fr/wfs/ows`, `data.geopf.fr/geocodage/search`.
Une page publiée sur claude.ai ne peut pas joindre ces services : le fond de carte fonctionne avec `dist/index.html`, en local ou sur GitHub Pages.

## Habillage

Bouton **Habillage** : un menu dédié à la présentation du plan, sans effet sur le calcul.

- **Véhicules** : un curseur règle la part des places occupées (0 à 100 %). Six gabarits vus de dessus, tirés au hasard selon leur fréquence : citadine, compacte, berline, break, SUV, utilitaire. Teintes du parc automobile, entrée en marche avant le plus souvent, léger décalage dans la place ; un véhicule ne déborde jamais de sa place.
- **Arbres** : une essence pour tout le plan (feuillu, pin parasol, olivier, palmier, conifère) ou un mélange, dans les îlots plantés et, en quinconce, dans les espaces verts imposés. Rotation et taille légèrement variées.
- **Nouveau tirage** redistribue véhicules et essences. Le tirage est stable : il ne change pas d'un affichage à l'autre.
- Couche **Habillage** dans le panneau des couches : calques Véhicules et Arbres.

Les symboles ont une seule source, `src/symbols.json` (générée par `tools/symbols.py`) : l'application les dessine et l'export DXF les insère comme blocs AutoCAD du même dessin.

## Places inclinées (épis)

Menu **Modifier**, « Inclinaison des places » : 90° (défaut), 75°, 60°, 45° ou 30° (paramètre moteur `slant`), pour les rangées intérieures et pour les places de bordure.

- **Place** : le rectangle largeur × longueur du réglage, pivoté de l'angle. Pas le long de l'allée `largeur / sin(angle)`, profondeur perpendiculaire `longueur × sin(angle) + largeur × cos(angle)` (4,67 m à 30°, 5,30 m à 45°, 5,58 m à 60°, 5,48 m à 75° pour 2,50 × 5,00). Neufert cote des profondeurs plus courtes à 45° et 60° (convention de débord sur bordure) : le moteur garde la géométrie du rectangle.
- **Voies** : largeur de voie de Neufert p. 272 selon l'angle et la largeur de place (interpolée entre 2,30 et 2,50 m) : 75° 5,00 m, 60° 4,00 m, 45° 3,00 m, 30° 3,00 m pour des places de 2,50 m. Le réglage « Largeur d'allée » ne vaut plus que pour l'anneau et le 90°. La voie sert alors de sens unique : les allées voisines sont parcourues en sens inverses, et chaque rangée double s'incline vers le sens de l'allée qu'elle borde (les sens sont imposés à l'analyse de circulation ; les repères du concepteur restent prioritaires). Une allée en impasse ne peut pas être à sens unique : elle est signalée.
- **Calcul** : chaque rangée double est étudiée dans un repère cisaillé où ses places redeviennent des rectangles ; les deux pentes alternent d'une rangée à l'autre. Les extrémités de rangée laissent des triangles, plantés.
- **Bordure** : chaque place est un parallélogramme dont le fond suit la limite (corde de `largeur / sin(angle)`) et dont l'avant est reculé de `profondeur × cotan(angle)` contre le sens de la limite. L'anneau devient une voie à sens unique de la largeur Neufert, parcourue dans le sens de la limite (les places restent à droite de la voie : anti-horaire sur le pourtour, horaire autour d'un bâtiment) ; ces sens sont imposés à l'analyse de circulation, sans repère visible. L'aire de manœuvre devant chaque place de bordure reste chaussée, et les rangées intérieures évitent les places de bordure et leur aire de manœuvre. Les accès, les cheminements devant les portes et la bande de bordure prennent la profondeur inclinée.
- **Sens de circulation** : le sens des voies n'est connu qu'en fin de calcul, et l'analyse peut s'écarter des sens imposés (l'anneau converge souvent vers un accès qui sert de sortie). Les places qui déboucheraient à contresens de leur voie à sens unique (axe d'entrée, de l'ouverture vers le fond, contre la circulation lue à 1 m devant l'ouverture) sont reposées dans un second calcul, inclinées dans l'autre sens si celui-ci convient : place par place en bordure, partie de demi-rangée reposée à pente inverse dans les rangées (chevron avec la demi-rangée voisine). En bordure, l'inclinaison retenue vaut aussi pour le jeu laissé de part et d'autre des accès, et une place qui ne tient pas dans son inclinaison (tête de colonne contre un angle) prend l'autre quand elle donne dans le sens d'une voie à sens unique et que la colonne reprend plus loin dans cette autre inclinaison. Le second calcul garde le réseau et les sens du premier. Au droit d'un carrefour où le sens change, ni l'une ni l'autre inclinaison ne convient : la place reste telle quelle. Calcul environ deux fois plus long quand il y a des places à corriger ; quelques places perdues aux changements d'inclinaison (ruisseau sans servitude à 30° : 304 à 301).
- **Places au sortir d'un virage** : une place dont l'entrée revient sur l'approche (voir « Entrée depuis l'amont » au test des places) et qui ne se prend pas d'un seul tenant en arrivant de l'amont (recherche de giration, Peugeot 3008, conducteur habile, voisines garées) est retirée : son emplacement reste planté, ou la rangée de bordure voisine déborde sur lui au droit d'un angle saillant (une place de plus en bout de rangée) ; contrôle repris sur le plan obtenu tant qu'il retire des places (trois fois au plus), avertissement « N places retirées ». Une place dont l'entrée ne revient pas sur l'une des voies qui arrivent au carrefour n'est pas concernée. Valable à toute inclinaison. L'interface affiche d'abord le plan calculé sans ce contrôle (paramètre moteur `turns: false`), puis le remplace quand le contrôle, lancé dans un Web Worker, retire des places (moins d'une seconde) ; l'export DXF calcule le plan complet. Plan de la capture du 8 novembre 2026 (ruisseau sans servitude, 30°) : 10 places retirées.
- **Effet sur le nombre de places** (scénario « rect », 302 places à 90°) : 295 à 75°, 276 à 60°, 248 à 45°, 184 à 30°. La bordure perd le plus (112 places à 90°, 107, 91, 72 puis 49) ; l'intérieur reste stable car l'anneau plus étroit libère de la profondeur.

## Giration

Menu **Giration** : épure tracée à la main, comme dans Covadis. « Tracer une trajectoire », puis toucher le plan : départ, points de passage ; le bouton Avant / Arrière pose un point de rebroussement (rond) et le tronçon suivant se fait en marche arrière (tirets). Terminer, Entrée ou double toucher lance la simulation ; Annuler retire le dernier point pendant le tracé. Les sommets se glissent ensuite sur le plan, la simulation suit.

- Véhicule de référence : Peugeot 3008 (3e génération, 2024), 4,54 × 1,90 m hors rétroviseurs, empattement 2,74 m, porte-à-faux 0,94 / 0,86 m, voie 1,63 m, garde au sol 0,198 m, pneus de 235 ; diamètre de braquage publié de 10,6 à 10,9 m selon la version, sans convention : 10,90 m lus entre trottoirs (11,95 m entre murs, calculé). Modèle bicyclette, épure de Jeantaud, volant à vitesse finie : roues tournées à 15°/s (habile), 10°/s (courant) ou 6°/s (prudent), échelle du CERTU reprise par Mensura, dans la limite de la direction (butée à butée en 4 s) ; braquage à l'arrêt aux seuls rebroussements, comme AutoTURN et Mensura, jamais pour le conducteur prudent.
- Le véhicule suit la trajectoire tracée (essieu avant en marche avant, essieu arrière en marche arrière) ; l'écart maximal et les tronçons non atteints sont signalés.
- Sorties : traces des huit flancs de pneus, enveloppe de carrosserie, marge de sécurité, véhicule à la position choisie (curseur, lecture), conflits typés : roue ou carrosserie contre bordure, mur (bâtiment) ou limite de parcelle ; la carrosserie ne heurte une bordure que si sa hauteur vue dépasse la garde au sol (0,198 m : une T2 de 0,16 m, trottoir compris, est survolée), et jamais celle d'un espace planté de 0,20 m au plus (Neufert p. 272, débord au-dessus d'un muret) : la caisse déborde au-dessus, sans alerte de marge, seuls les pneus butent. Une marge franchie est une alerte.
- Hypothèses (repliées sous le résultat) : conducteur Prudent, Courant ou Habile (par défaut), vitesse (5 km/h par défaut), marge (0,10 m), bordures T2 ou T3 et leur hauteur vue (0,16 m par défaut, Métropole Aix-Marseille-Provence).
- Bordures : contours de la surface roulable (chaussée, allées, accès, fonds de rangées, places) à l'intérieur de la parcelle, îlots des rangées compris. Bordure d'espace planté quand la pleine terre la longe (espaces plantés, îlots, noues, bassins, reculs, servitudes, comme dans le calcul des espaces verts) ; de trottoir le long d'un cheminement piéton ou d'un bâtiment. Les angles vifs des places inclinées, où la bordure ne mesure que quelques millimètres, suivent la même règle. Le plan affiché est contrôlé tel quel.
- **Test de toutes les places** (« Tester toutes les places ») : chaque place est jouée en entrée, en marche avant depuis la voie 10 m en amont, puis en sortie, en marche arrière vers l'allée, parking plein (toutes les places occupées, les deux voisines garées au pire, décalées de 0,10 m vers la place testée). La voiture garée est centrée sur l'axe de la place (du milieu du fond au milieu de l'ouverture, places en coin comprises), nez vers le fond : pare-chocs à 2 cm de la ligne quand le fond est une autre place (places jointes) ou un espace planté, à la marge plus 2 cm d'une bordure de trottoir, d'un mur ou d'une limite. La marge de sécurité ne vaut pas contre les voitures garées : places jointes, seul le contact compte. Manœuvre planifiée par A* hybride depuis la pose garée (le modèle bicyclette est réversible : une manœuvre parcourue à rebours en est une), dans l'espace position, cap et angle des roues : le volant tourne en roulant à la vitesse du profil, d'où des raccords en clothoïde et des traces de pneus continues ; braquage à l'arrêt aux seuls rebroussements. Recherche d'un seul tenant, puis avec une reprise au plus, puis sans limite : le moins de reprises possible.
- **Statuts** : une manœuvre est possible (trouvée sans contact), impossible (la voiture garée ne tient pas, ou le domaine de recherche a été exploré en entier sans solution : pas de 0,25 m, 5°, zone de 16 m) ou non conclue (budget de recherche épuisé, réglable). Un échec de recherche n'est jamais présenté comme une impossibilité.
- **Entrée depuis l'amont** : chaque entrée part de la voie, 10 m en amont de la place le long de la circulation (à travers virages et carrefours), voiture alignée sur la voie dans le sens de la circulation ; au droit d'un virage, où une voiture alignée à 10 m chevaucherait les deux voies, le départ à 15 m est aussi accepté. Le plus souvent, la voiture suit la voie jusqu'au départ de la sortie parcourue à rebours, sans braquage à l'arrêt à la jonction ; sinon (virage, carrefour), l'entrée est cherchée depuis l'amont. Arriver en marche avant puis reculer avant d'entrer compte deux reprises. Les pneus ne passent jamais une bordure : ils restent sur la chaussée et les places, jamais sur la pleine terre. À défaut d'entrée, la place est rouge ou grise, motif « entrée impossible en arrivant de l'amont » ou « entrée depuis l'amont non conclue ». Budget de la recherche sans limite de reprises : 150 000 états.
- **Verdicts** : vert, d'un seul tenant ; bleu, avec reprises (3 au plus) ; orange, sous condition (marge entamée ou plus de 3 reprises) ; rouge, impossible ; gris, non conclu. À l'écran, vertes et bleues partagent la pastille verte. Repères doublés d'une forme : rond, triangle, carré, anneau. Marge minimale mesurée (distance de la caisse aux obstacles). Calcul réparti sur plusieurs Web Workers (un par cœur disponible, quatre au plus), par lots ; toucher une place montre sa manœuvre, ou la moins mauvaise si aucune n'est possible. Un plan modifié efface les verdicts.
- **Collisions continues** : entre deux poses, la caisse balaye au plus l'enveloppe convexe de ses deux empreintes agrandie de la flèche de l'arc décrit par son coin le plus éloigné du centre de rotation ; ce contrôle exact par excès remplace l'échantillonnage (épure et test des places). La marge, simple alerte, reste contrôlée aux échantillons. Les pneus, seuls arrêtés par une bordure d'espace planté, sont contrôlés à l'angle réel des roues, élargis de 1 cm pour couvrir leurs traces entre deux poses.
- Calques `Trajectoire et roues`, `Carrosserie`, `Marge de sécurité`, `Conflits`, `Verdicts des places` ; après le test, les places vertes (avec ou sans reprises) n'ont qu'une pastille verte, les orange et les rouges montrent en plus la caisse balayée et les traces des pneus de leur entrée (calques `Carrosserie` et `Trajectoire et roues`), ou la caisse garée si elle ne tient pas ; export DXF sur `GIR_TRAJECTOIRE`, `GIR_ROUES`, `GIR_CARROSSERIE`, `GIR_MARGE`, `GIR_VEHICULE` (gabarit tous les 2 m), `GIR_CONFLITS`.

## Export tableur (XLS)

Menu **Exporter**, bouton « Télécharger le tableur XLS » : classeur XML Spreadsheet 2003 (`.xls`, lu par Excel, LibreOffice et Numbers, sans bibliothèque). Feuille **Synthèse** : projet (dates de création et de dernière modification, conservées dans le navigateur avec le projet), emplacement (commune, références cadastrales, latitude et longitude WGS84 du centre de la parcelle), places (total, standard, PMR, IRVE, bordure et cœur, conformités, parts), arbres, surfaces et ratios (enrobé, espaces verts, m² par place, places par hectare), hypothèses de calcul et estimation financière (tranche, ratios, totaux mini, maxi et central, avec ou sans études et MOE). Feuille **Barème** : le barème en vigueur et la tranche appliquée.

## Export DXF

Bouton **Exporter** : fichier DXF AutoCAD 2010 (texte UTF-8), unités en mètres, au choix en **RGF93 / Lambert-93** (EPSG:2154) ou en **RGF93 / CC** de la zone du site (EPSG:3942 à 3950, CC44 à Nice).

- Chaîne de coordonnées : repère local → Web Mercator → RGF93 géographique → conique conforme de Lambert sur l'ellipsoïde GRS80. Projections vérifiées contre PROJ à moins d'un millimètre ; sommets cadastraux restitués au centimètre.
- Un calque AutoCAD par calque de l'application ; les calques masqués sont exportés éteints. Le cadastre est limité à l'emprise.
- Entités : `LWPOLYLINE` (contours), `HATCH` (trames : végétation, bâti, servitudes, places PMR), `TEXT` (numéros de parcelle, sections, cartouche), `INSERT` du bloc de marquage `SIG_FLECHE` et des blocs d'habillage : `VEH_CITADINE`, `VEH_COMPACTE`, `VEH_BERLINE`, `VEH_BREAK`, `VEH_SUV`, `VEH_UTILITAIRE`, `ARB_FEUILLU`, `ARB_PIN`, `ARB_OLIVIER`, `ARB_PALMIER`, `ARB_CONIFERE`. Les blocs sont dessinés en vraie grandeur (véhicules) ou au rayon unitaire (arbres, insérés à l'échelle du houppier), sur le calque 0 en couleur DUBLOC ; leur orientation tient compte de la convergence des méridiens.

| Couche | Calques AutoCAD |
|---|---|
| Projet | `PRJ_PARCELLE`, `PRJ_RECUL`, `PRJ_VOIRIE`, `PRJ_SIGNALISATION`, `PRJ_PLACES`, `PRJ_PLACES_PMR`, `PRJ_PLACES_IRVE`, `PRJ_PLACES_IRVE_EQ`, `PRJ_ILOTS`, `PRJ_NOUES`, `PRJ_BATIMENTS`, `PRJ_ESPACES_VERTS`, `PRJ_BASSIN_TALUS_HAUT`, `PRJ_BASSIN_TALUS_BAS`, `PRJ_SERVITUDES` |
| Habillage | `HAB_VEHICULES`, `HAB_ARBRES` |
| Occupation du sol | `OCC_VOIRIE`, `OCC_BATI`, `OCC_VEGETATION` |
| Cadastre | `CAD_PARCELLES`, `CAD_NUMEROS`, `CAD_FEUILLES`, `CAD_COMMUNES` |
| Giration | `GIR_TRAJECTOIRE`, `GIR_ROUES`, `GIR_CARROSSERIE`, `GIR_MARGE`, `GIR_VEHICULE`, `GIR_CONFLITS` |
| Repères | `EMPRISE`, `INFO` (système de coordonnées, sources, bilan) |

Le squelette du fichier (en-tête, tables, calques, types de ligne, blocs) est produit par ezdxf (`tools/dxf_template.py`) ; chaque export est relu par ezdxf dans les tests, sans erreur d'audit.

## Valeurs par défaut

| Paramètre | Valeur | Source |
|---|---|---|
| Place | 2,50 × 5,00 m | usage courant PLU (minimum NF P 91-100 : 2,30 × 5,00) |
| Verdure autour du bassin | 2,00 m | bande verte d'accès et d'entretien autour du haut de talus, sans stationnement |
| Hauteur bassin de rétention | 1,50 m | talus 3 pour 1 : pied de talus à 3 × la hauteur du haut de talus |
| Noue entre rangées | aucune (0) ; 1 à 6 m | bande plantée entre les deux demi-rangées dos à dos de chaque rangée double ; elle allonge la période des rangées de sa largeur, reste un espace vert et est interrompue par les allées transversales et s'arrête là où le cœur s'arrête (elle ne déborde jamais sur la chaussée) ; elle est figurée en haricot allongé aux bouts arrondis (haut de talus en trait fort, barbules courtes de longueurs alternées vers l'intérieur, fond en un seul trait) et exportée sur le calque `PRJ_NOUES` ; une valeur entre 0 et 1 m équivaut à aucune noue, au-delà de 6 m à 6 m |
| Largeur d'allée | 6,00 m, double sens | usage courant PLU ; double sens à partir de 5,00 m (NF P 91-120, 5,50 m en NF P 91-100), sens unique en dessous |
| Largeur voie en sens unique | 5,00 m | NF P 91-100, places à 90° (2,80 m minimum pour une voie de circulation en NF P 91-120) |
| Largeur de l'entrée | 6,00 m | accès véhicules ; réglable (usage courant : 3,00 m minimum en sens unique, 5,00 à 6,00 m à double sens) |
| Largeur d'entrée du bâtiment | 3,00 m | seuil et couloir libre devant l'entrée (largeur) ; réglable |
| Place PMR | 3,30 × 5,00 m, 2 % (min. 1) | arrêté du 20 avril 2017 (ERP) |
| IRVE | 20 % pré-équipées, dont 2 % PMR (min. 1) | CCH L111-3-4, non résidentiel neuf |
| IRVE équipées | 1 point de charge par tranche de 20 places (5 %, min. 1), dont une PMR ; réglable | parkings de plus de 20 places, non résidentiel ouvert au public |

Tout est réglable dans le panneau Paramètres. À vérifier selon le PLU et la destination du projet.

## Structure

```
src/geom.rs       vecteurs, quadrilatères, régions pair-impair, intervalles de bande exacts
src/region.rs     reculs par arête, obstacles, érosions (booléens robustes via la crate geo)
src/perimeter.rs  places en bordure, bords droits et courbes
src/rows.rs       optimisation orientation × décalage, découpe des rangées doubles, allées resserrées
src/finish.rs     connectivité depuis les accès, PMR, IRVE
src/circulation.rs réseau des voies, sens de circulation (typologies imposées comprises), flèches au sol
src/planted.rs    espaces plantés, arrondis en bordure de chaussée
src/occupation.rs occupation du sol : voirie, bâti, végétation
src/proj.rs       projections Lambert-93 et CC (RGF93, GRS80)
src/dxf.rs        export DXF AutoCAD 2010 géoréférencé
src/giration/     simulateur de giration : véhicule (vehicle.rs), cinématique (kinematics.rs), bordures, murs et
                  limites (scene.rs), suivi de trajectoire (epure.rs), traces, enveloppes et conflits (sweep.rs),
                  A* hybride (planner.rs), test des places (places.rs), contrat JSON (api.rs)
src/dxf_template.dxf squelette AutoCAD (calques, types de ligne, blocs d'habillage)
src/symbols.json  symboles d'habillage (véhicules, arbres) et flèche au sol, partagés par l'interface et l'export
src/lib.rs        orchestration, union de parcelles, interface WebAssembly (JSON en entrée, JSON en sortie)
tests/quality.rs  invariants : aucun chevauchement, aucune place hors zone ou sans accès, quotas, 60 parcelles aléatoires,
                  flèches dans la chaussée, sens unique fortement connexe, bascule des voies par le squelette,
                  voies resserrées à la largeur sens unique, repères stables, sens de parcours imposé
tests/giration.rs véhicule, cinématique, scène, suivi, conflits ; recettes AASHTO P, NF P 91-100, Neufert
examples/giration.rs rendu SVG de contrôle d'un virage de la Peugeot 3008
examples/places_bench.rs test de toutes les places d'un scénario : durée et verdicts
examples/audit_aisle.rs  allée de 6 m entre places à 90° : verdict selon le véhicule, la marge et l'occupation
examples/audit_plan.rs   places d'un plan exporté de l'application : verdicts selon le diamètre lu, la marge et le véhicule
examples/audit_moves.rs  places d'un plan exporté : sens de marche de chaque tronçon, roues tournées à l'arrêt
examples/plan_bench.rs   toutes les places d'un plan exporté : verdicts, reprises, durée par place
examples/dxf.rs   export d'un scénario en DXF
web/app.html      interface (canvas, poignées, panneaux, mode carte, couches et calques, export)
tools/bundle.py   assemble dist/index.html avec le moteur embarqué
tools/wrap_w2j.py repli JavaScript du moteur (wasm2js)
tools/symbols.py  génère src/symbols.json
tools/dxf_template.py génère src/dxf_template.dxf (ezdxf) avec les blocs de src/symbols.json
```

## Compiler

```sh
rustup target add wasm32-unknown-unknown
cargo test --release          # invariants de qualité
./build.sh                    # produit dist/index.html (wasm-opt utilisé s'il est installé)
```

Ouvrir `dist/index.html` dans un navigateur, ou le publier tel quel sur GitHub Pages.

Scénarios de contrôle : `cargo run --release --example run -- scenarios/ruisseau.json sortie.json`, puis `python3 tools/render.py sortie.json plan.png`.

**Sélection et suppression** : un élément sélectionné (sommet, accès, bâtiment, espace vert, bassin, zone non aedificandi) porte une petite icône de suppression à proximité ; sur un sommet d'objet, elle retire ce sommet (au moins trois sommets restent), sinon l'objet entier. La barre d'actions n'apparaît plus que pour une parcelle cadastrale (Étudier, Ajouter à l'étude) et un bâtiment (forme en L, pivoter). Toutes les icônes, sur le plan comme dans les menus, ont la taille de celle des accès (18 px).

**Boutons d'un segment** (survol ou toucher d'un contour) : « + » ajoute un sommet au milieu, le **rond** du centre déplace le segment entier en le glissant (ses deux sommets suivent, les autres restent), et le dernier bouton arrondit un segment droit (icône d'arc) ou redresse un segment courbe (icône de ligne, bouton plein).
