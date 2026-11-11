# Corpus des rampes de parking

Objet : choisir la forme des rampes d'un parc en ouvrage d'après son fonctionnement global, avec les conditions de giration à l'entrée et à la sortie, sur des exemples construits. Les types implantés dans Parkimo sont signalés. Valeurs « calculé » : dérivées géométriquement, absentes des sources.

## 1. Ce qui décide du type de rampe

| Critère | Options | Effet sur les rampes |
|---|---|---|
| Sens de circulation sur les plateaux | boucle à sens unique ; allées à double sens | La boucle fixe le sens d'entrée et de sortie de chaque rampe : on y entre et on en sort dans le sens du flux. |
| Montée et descente | séparées ; confondues (double sens) | Séparées : deux rampes à sens unique, flux sans croisement. Confondues : une voie de 5,50 m droite (NF P 91-100 § 3.3), 6,50 m chasse-roues compris (Neufert). |
| Recherche de place | circuit qui longe toutes les places ; accès direct à chaque niveau | Rampes aux deux bouts : le circuit parcourt chaque niveau. Rampe en tour ou en hélice indépendante : chaque niveau sans traverser les autres. |
| Vidage | sortie par le même circuit ; sortie rapide | Une rampe de descente courte, ou placée au centre, raccourcit la sortie (gares, stades, parcs relais). |
| Évolution du bâtiment | plateaux réversibles | Rampes hors des plateaux (tour, hélice, rampes démontables) : les plateaux peuvent changer d'usage. |

## 2. Types

### 2.1 Plateaux horizontaux et rampes droites séparées (implanté)

Rampes à sens unique, montée à un bout, descente à l'autre, superposées d'un niveau à l'autre ; boucle à sens unique sur chaque plateau. Neufert classe ce système en tête (rampes droites de 15 % au plus, raccord extérieur à 10 %) ; la NF P 91-100 admet 18 % avec des raccords R 10 en sommet et R 15 en pied.

- **En rangée, le long des allées.** La rampe remplace le début d'une rangée de façade. On l'aborde par l'allée transversale en venant d'une autre allée (deux quarts de tour), on en sort en S dans l'allée voisine. Deux modules au moins.
- **En pignon, en travers des allées.** La rampe occupe la bande de pignon contre l'allée transversale. On y entre depuis l'allée la plus éloignée, on en sort dans l'allée opposée (un quart de tour à chaque bout). Trois modules au moins.
- Variante non implantée : rampe droite unique à double sens (petits parcs, 5,50 à 6,50 m de large). Le silo de la ZAC Georges Besse 2 à Nîmes a des rampes de 7 m de large pour un bon rayon de braquage ; celles des niveaux 3 et 4 sont démontables pour passer ces niveaux en bureaux.

### 2.2 Demi-niveaux décalés, système d'Humy (implanté)

Deux demi-plateaux décalés d'un demi-étage, reliés aux deux bouts par des demi-rampes courtes ; circulation en boucle, montée par un côté, descente par l'autre. Neufert : rampes de 13 à 15 % sur 10 m environ, décalage de 1,50 m environ par demi-niveau. Le plus économe pour une dalle de 32 m (deux modules), largeur courante de ce type.

- Darmstadt, parking GE 1a (Lengfeld und Wilisch) : 17 demi-niveaux, demi-rampes à sens unique pour la montée et la descente.
- Darmstadt, TU Lichtwiese : 16 demi-niveaux, demi-rampes à double sens, toutes les places sur le circuit de recherche.
- Darlington, gare (Napper) : une allée de 6 m par niveau, places de 4,80 m de part et d'autre ; demi-niveaux de 1,65 m (calculé d'après les cotes +47,60, +49,25, +50,90, +52,55) ; rampes à 1:12 et 1:13 ; descentes rapprochées au centre pour raccourcir la sortie.
- Variante « quick return » : une rampe de plus au bout de la sortie ; on monte par le long chemin, on descend par le court.

Dans Parkimo : deux demi-plateaux d'un module, décalés d'un demi-étage, et à chaque bout une demi-rampe en U à double sens, sur le modèle de TU Lichtwiese (toutes les places sur le circuit). Allées à double sens ; on circule à droite, si bien que la montée tourne deux fois à droite par la voie intérieure et la descente deux fois à gauche par la voie extérieure, sans croisement.

### 2.3 Rampes en ciseaux (non implanté)

Deux rampes à sens unique côte à côte, de sens opposés, qui se croisent en plan d'un niveau à l'autre. Elles doivent être côte à côte : superposées dans la même bande, elles se rencontreraient en bout de rampe. Le manuel de conception britannique consulté les range parmi les demi-niveaux (« SLD 3, side-by-side ramps, scissors type »).

### 2.4 Plateaux inclinés (implanté)

Les plateaux sont eux-mêmes la rampe ; on se gare le long de la pente, sans perte de place pour les rampes. Pente limitée par les places : 5 % sur l'axe d'une place, 7,5 % au plus (NF P 91-100 § 3.4) ; Neufert : 6 %. Plusieurs variantes en simple ou double hélice.

- Chambéry, parking Ravet (Hérault Arnod) : 499 places ; on monte depuis la rue sur des rampes en légère pente qui servent de plateaux autour d'un patio planté, on redescend par une spirale cylindrique de l'autre côté ; boucle à sens unique, flux montant et descendant sans croisement.
- Spiral car park : 16 m de large et 57 m de long au moins, on monte et on descend sur les mêmes rampes.

Dans Parkimo : deux modules en pente de sens opposés, chacun d'un demi-étage sur la longueur des rangées ; à chaque bout, ils se rejoignent au même niveau par un demi-tour horizontal à double sens. Le parcours forme une hélice continue, montée et descente sur les mêmes allées en circulant à droite, toutes les places sur le circuit.

### 2.5 Rampes hélicoïdales (implanté)

Hélice simple à double sens, ou double hélice : une pour monter, une pour descendre, imbriquées ou concentriques. Accès direct à chaque niveau, plateaux libres, mais emprise et coût élevés (Neufert). Conditions : voie courbe à sens unique de 4,00 m, rayon extérieur d'au moins 8,50 m, pente de 18 % au plus mesurée dans l'axe (NF P 91-100 § 3.3 et 3.4) ; Neufert : dévers de plus de 3 %, bord intérieur à plus de 5 m, une voie de 4,00 m entre R 5,00 et R 9,00 (5,50 m avec mur et bordure), deux voies 8,60 m.

- Grenoble, parking de GaP Architectes : 469 places sur 5 niveaux ; double hélice centrale « type Chambord » ; chaque niveau sans passer par celui du dessous ni croiser d'autre flux ; plateaux reconvertibles en bureaux ou activités.
- Bron, parc relais Mermoz-Pinel (Gautier Conquet) : 418 places sur 4 niveaux sans demi-paliers ; double rampe hélicoïdale, extérieure pour descendre, intérieure pour monter, pente de 12 % au plus, rampes plus larges que nécessaire.
- Lyon, garage Atlas : hélice à double révolution de 12 %, montée de 6 m imbriquée avec la descente, noyau d'ascenseurs et d'escalier au centre.
- Lyon, parking des Célestins (Targe, Wilmotte, Buren) : 439 places sur 6,5 niveaux, rampe hélicoïdale autour d'un puits central.
- Gif-sur-Yvette, silo du campus Paris-Saclay (GaP Architectes, Un1on) : 317 places sur 5 niveaux, plateaux horizontaux, rampe circulaire à l'angle nord-est.

Hélice unique à double sens (implanté, « Hélice à double sens ») : une seule tour au premier bout, à deux voies côte à côte (Neufert, rampe courbe à deux voies : parapet R 4,50, voie intérieure de 4,00 m entre R 5,00 et R 9,00, voie extérieure de 3,10 m entre R 9,50 et R 12,60, bordure R 13,10). On tient sa droite : la voie intérieure monte, l'extérieure descend (comme à Bron). Un tour par niveau, porte au même endroit, sur l'allée transversale ; à la porte, chaque sens coupe l'autre, ce qui demande une circulation bien réglée sur les plateaux (exemple : Antibes, parking du Pré-aux-Pêcheurs, Q-Park, 600 places sur 3 niveaux enterrés, prix européen 2015, d'après la description de l'utilisateur ; plan non publié). Fonctionne enterré comme en élévation, sur deux modules (Antibes, d'après l'utilisateur). Dans Parkimo : l'allée transversale du bout de la tour est élargie à 2 × (11,05 − 7,00) = 8,10 m pour que ses deux voies, au quart de sa largeur, soient dans le prolongement des axes des deux voies de la tour : voie ouest (on descend en v) sur la voie intérieure qui monte, voie est sur la voie extérieure qui descend. Pas de S : on monte en venant de l'allée la plus haute (quart de tour), on sort vers la plus basse ; on descend en venant de la plus basse, on sort vers la plus haute. Les deux sens se croisent aux carrefours de l'allée transversale et des allées. La descente demande deux quarts de tour entre la voie nord de l'allée basse et la voie sud de l'allée haute : 2 × 6,50 = 13,00 m, quand deux modules en offrent 23,20 − 9,20 = 14,00. Bande de 0,30 + 13,10 + 7,00 − 2,025 + 8,10 = 26,475 m au premier bout, bande ordinaire au second. Pente sur la voie intérieure : 2,90 / (2π × 7,00) = 6,6 % ; 4,2 % sur l'axe de la voie extérieure. Sur 32,40 × 80 m, où deux tours séparées ne tiennent pas : 72 places par niveau courant ; sur 48,40 × 80 m : 116 contre 96 avec deux tours.

Trois autres places de la même tour (implantées, d'après les schémas validés par l'utilisateur) :

- **Au centre** (« Double sens au centre ») : la tour au milieu des rangées, centrée entre deux allées qui passent de part et d'autre et bouclent la circulation ; elle coupe les allées qu'elle croise. Portes à l'ouest, sur une allée des portes de 8,10 m (voie est vers le nord sur la voie intérieure qui monte, voie ouest vers le sud sur l'extérieure qui descend) ; on monte en venant de l'allée basse (quart de tour à droite), on sort vers l'allée haute ; on descend en venant de l'allée haute, on sort vers l'allée basse. Allée transversale de 6,00 m à l'est de la tour, bord est sur la grille des places. Il faut 26,20 m entre les deux allées qui l'encadrent, à 0,50 m d'empiètement près (NF P 91-100, tab. 1, catégorie 2 : réduction de la somme allée + longueur de place), soit trois modules (26,00 m), et une bande de 11,05 + 2,025 + 13,10 + 6,00 = 32,175 m au milieu des rangées, une place au moins de part et d'autre. Sur 48,40 × 100 m : 170 places par niveau courant (198 en rangée).
- **En anneau** (« Double sens en anneau », plan carré) : la même tour contre l'allée transversale du premier bout, élargie à 8,10 m pour servir d'allée des portes ; au-delà, l'allée du second bout ou, s'il reste des rangées, une allée transversale. Sur 48,40 × 44,50 m, le parking entoure la tour : 50 places par niveau courant, places de façade et de pignon.
- **En façade** (« Double sens en façade ») : la tour au milieu d'une façade longue, en grande partie hors de la dalle (centre à 1,875 m au-delà sur deux modules). L'allée de rive s'élargit de 6,00 à 8,10 m devant elle (sur la rangée de façade) pour que ses deux voies soient tangentes, au sud de la tour, aux deux voies de la tour : voie nord vers l'ouest sur la voie intérieure qui monte, voie sud vers l'est sur l'extérieure qui descend. On y entre et on en sort dans le sens de l'allée, sans quart de tour, avec un S de raccord (7,05 m pour 2,075 m de décalage) ; les allées transversales bouclent d'une moitié du plateau à l'autre, d'où deux allées au moins. Sur 32,40 × 80 m : 94 places par niveau courant (72 avec la tour au bout). Comme les hélices hors dalle, choix de fonctionnement que le mode Auto ne retient pas ; une alerte signale la tour qui sort de la zone constructible.

Hors de la dalle (implanté, « Hélices hors dalle ») : les mêmes tours, à la même place, mais la dalle s'arrête à la file de poteaux derrière l'allée transversale (retrait de 0,20 + 21,80 − 6,00 − 0,30 − 0,20 = 15,50 m à chaque bout pour des allées de 6,00 m) ; la tour, adossée au pignon, n'entre dans la dalle que par sa porte, sur 4,00 m. Plateaux rectangulaires, libres et réversibles, sans places de pignon : sur 48,40 × 80 m, 84 places par niveau courant contre 96 avec les tours dans la dalle, pour la même emprise. Choix de fonctionnement, que le mode Auto ne retient pas.

Dans Parkimo : deux tours hélicoïdales à sens unique sur plateaux horizontaux, montée à un bout et descente à l'autre (le principe de Bron, une voie par sens, mais dans deux tours), au gabarit Neufert de la rampe courbe à une voie. Analyse Parkimo, pour des plateaux horizontaux : dans une hélice à double sens, l'un des sens coupe l'autre à chaque porte ; avec deux hélices concentriques, il faut traverser la voie extérieure pour atteindre l'intérieure ; deux hélices imbriquées (Chambord) font alterner les portes d'un côté à l'autre de la tour d'un niveau au suivant. Deux tours séparées gardent une porte au même endroit à chaque niveau, dans le sens de la circulation.

### 2.6 Rampes centrales de distribution, accolées (retiré)

Rampes au cœur du plateau qui séparent les flux montants et descendants, noyaux aux angles. Sur un plan carré, où les rangées sont trop courtes pour une rampe en rangée et où les rampes en pignon prennent les deux bouts, montée et descente se collent l'une à l'autre au milieu.

- Angers, silo de l'îlot Académie : 300 places sur quatre niveaux, deux rez-de-chaussée à deux altitudes de rue.
- Nice, parking Sulzer (Jean-Michel Battesti, Interparking, 2013) : terrain de 2 484 m², 6 niveaux de sous-sol, 455 places, 13 217 m² de SHON ; rampes accolées sur plan carré (exemple donné par l'utilisateur, plan non publié).

Retiré de Parkimo : d'après l'utilisateur, des rampes droites accolées au centre du plateau ne se rencontrent pas ; au centre, c'est la tour à double sens qui se place (§ 2.5).

## 3. Girations retenues dans Parkimo

- **Gabarit.** NF P 91-100 § 3.3, voie à sens unique : 4,00 m en courbe, rayon extérieur 8,50 m au moins ; 3,00 m en ligne droite. Hypothèse : la trajectoire suit le milieu de la voie, au rayon 6,50 m. L'épure est la bande balayée : 3,00 m sur les droites, de 4,50 à 8,50 m du centre sur les courbes.
- **Contrôle.** Pour chaque entrée et chaque sortie : aucun poteau, aucune autre rampe, rien hors du nu intérieur des façades. Les poteaux dans la voie de rampe et ses abords sont retirés (la structure propre des rampes reste à faire) ; une épure gênée est dessinée en rouge et signalée.
- **Places par niveau.** Les emprises des rampes sont prises à tous les niveaux (au dernier, la montée qui arrive laisse une trémie). Les places dans les épures de l'extrémité basse (entrée de la montée, sortie de la descente) reviennent au dernier niveau ; celles de l'extrémité haute (sortie de la montée, entrée de la descente) reviennent au rez.
- **Conditions (calculé, allées de 6,00 m, voie de rampe de 3,25 m, rampe de 18,36 m pour 2,90 m d'étage à 18 %) :**
  - en rangée : bouche à 8,50 − 6,00 = 2,50 m du début des rangées ; allée d'approche dont la trajectoire passe à 13,00 m au moins de l'axe de la voie (deux quarts de tour de 6,50 m), donc une seconde allée ; sortie en S de 8,46 m jusqu'à 1,50 m dans l'allée ; rangées d'au moins 2,50 + 18,36 + 8,46 = 29,32 m ;
  - en pignon : 18,36 + 2 × 6,50 = 31,36 m entre les trajectoires des allées d'entrée et de sortie, soit 32,00 m pour trois modules (34,00 m en serrant les trajectoires à 2,00 m des bords) ; deux modules n'offrent que 18,00 m ;
  - hélice : un tour par niveau, si bien que la porte de chaque niveau est au même endroit ; on y entre et on en sort dans le sens de la voie, tangente à la voie ouest de l'allée transversale (circulation à droite, voie au quart de l'allée). Tour contre la file de façade : 0,30 + 9,50 + 9,00 − 1,50 − 1,50 + 6,00 = 21,80 m de bande d'extrémité, places de pignon contre l'allée transversale. La voie de l'allée passe sur la tour sur √(9,50² − 6,00²) = 7,37 m de part et d'autre de la tangence (la porte) et entre dans la tour, comme pour la tour à double sens : avant la porte, un quart de tour depuis l'allée la plus haute, après, un quart de tour vers la plus basse, qui peuvent aller jusqu'à la porte : 2 × 6,50 = 13,00 m entre leurs trajectoires, que deux modules offrent (18,00 m) ; un seul n'en offre que 2,00. Sur 32,40 × 80 m : 64 places par niveau courant avec les tours dans la dalle, 56 hors de la dalle, 72 avec une tour à double sens. Pente dans l'axe de la voie (R 7,00) : 2,90 / (2π × 7,00) = 6,6 %. Les tours se superposent d'un niveau à l'autre, à une hauteur d'étage d'écart ;
  - plateaux inclinés : même plan que les demi-niveaux (joint de 0,50 m, zones d'extrémité de 11,80 m), mais les demi-tours sont horizontaux et les modules en pente : un demi-étage sur la longueur des rangées, 1,45 / 35,00 = 4,1 % pour 14 places par rangée et 2,90 m d'étage. L'axe des places à 90° est horizontal ; leur plus grande pente est celle du plateau, limitée à 7,5 % (NF P 91-100, § 3.4), soit 19,33 m de rangées au moins ; au-delà de 6 %, Neufert conseille d'allonger. Places PMR sur une partie horizontale (dévers de 2 % au plus, arrêté du 20 avril 2017) ;
  - en demi-niveaux : courbe à double sens de la NF (voie intérieure de 3,50 m au rayon extérieur 8,50 m, voie extérieure de 3,00 m jusqu'à 11,50 m) ; trajectoires au milieu des voies, rayons 6,75 et 10,00. Les voies intérieures des deux allées (à 1,50 m de leur bord côté dos à dos) doivent être à 2 × 6,75 = 13,50 m : 10,00 m de places dos à dos et 3,00 m de demi-voies n'en font que 13,00, d'où un joint de 0,50 m entre les demi-plateaux. Zone d'extrémité de 0,30 + 11,50 = 11,80 m (file de façade et demi-rampe), sans places de pignon. La voie extérieure, plus large en courbe, s'écarte de 0,25 m de son axe en ligne droite dans la zone d'extrémité. Pente mesurée dans l'axe de la voie intérieure : 1,45 / (π × 6,75) = 6,8 % pour 2,90 m d'étage. Les demi-rampes se superposent d'un niveau à l'autre, à une hauteur d'étage d'écart.
- **Pourquoi les rampes en pignon du lot 2 ne passaient pas.** Elles commençaient et finissaient aux angles de façade : il fallait un demi-tour dans 9,25 m (allée de 6,00 m et voie de 3,25 m), quand l'épure NF en demande 2 × 8,50 = 17,00 m.
- **Choix automatique.** Parkimo construit les huit implantations dans la dalle et garde celle qui laisse le plus de places par niveau courant. Sur 32,40 × 60 m : en rangée seulement (60 places par niveau courant, 64 au rez et au dernier ; les demi-niveaux demandent 32,90 m). Sur 48,40 × 60 m : 102 en rangée contre 98 en pignon. Sur 33 × 100 m : 124 en rangée contre 120 en demi-niveaux. Sur 48,40 × 80 m : 96 places par niveau courant avec les tours hélicoïdales (100 au rez et au dernier). Les plateaux inclinés donnent autant de places que les demi-niveaux (même plan) : le choix entre les deux tient au fonctionnement (rampes courtes et plateaux horizontaux, ou plateaux en pente sans rampe), pas au nombre de places. La recherche de dalle fait passer les emprises où les rampes tiennent avant les autres.

## 4. Pistes

1. Tour hélicoïdale hors de la dalle : bande arrière des tours pour les noyaux (escaliers, ascenseurs) ; rampes droites accolées hors de la dalle, le long d'une façade.
2. Sortie rapide : descente courte au centre ou rampe de retour ; demi-niveaux à plus de deux demi-plateaux.
3. Accès depuis la voirie : palier à 5 % au plus sur 4,00 m (NF P 91-100 § 3.4), giration d'entrée sur le site.

## Sources

- NF P 91-100 (mai 1994), § 3.3 et 3.4 ; Neufert, éd. française, p. 274-276 (documents fournis).
- [GE1a Multi-Storey Car Park, Darmstadt](https://german-architects.com/en/lengfeld-and-wilisch-architekten-darmstadt/project/ge1a-multi-storey-car-park) et [description des rampes](https://catalan-architects.com/en/lengfeld-and-wilisch-architekten-darmstadt/project/ge1a-multi-storey-car-park)
- [Parking garage TU Lichtwiese](https://www.american-architects.com/en/lengfeld-and-wilisch-architekten-darmstadt/project/parking-garage-tu-lichtwiese)
- [Darlington Station, Design and Access Statement, part 2](https://www.darlington.gov.uk/media/14968/sd11a-design-and-access-statement-gateway-east-part-2.pdf) et [rapport de concertation](https://www.darlington.gov.uk/media/14951/sd41-planning-application-consultation-report.pdf)
- [Car park layouts (split level, quick return, spiral)](https://www.scribd.com/doc/95131371/Layouts)
- [Manuel de conception de parkings, table des matières (SLD, SD, FSD, VCM)](https://www.scribd.com/doc/257185487/FULL-HANDBOOK)
- [Parking de Grenoble, GaP Architectes](https://www.architonic.com/es/project/gap-architectes-parking-building-in-grenoble/5102742)
- [Q-Park Pré-aux-Pêcheurs, Antibes, dossier du prix européen 2015](https://europeanparking.eu/wp-content/uploads/2024/12/power-point-presentation-category-1-new-antibes-pre-aux-pecheurs.pdf)
- [Parc relais Mermoz-Pinel, Bron](https://divisare.com/projects/310158-gautier-conquet-architectes-parc-relais-mermoz-pinel-bron-france)
- [Ancien garage Atlas, Lyon](https://monumentum.fr/architecture-contemporaine-remarquable/ACR0000134/lyon-3e-arrondissement-immeuble-a-logements-dit-immeuble-de-lancien-garage-atlas/edit-localisation)
- [Parking des Célestins, Lyon](https://www.visiterlyon.com/sejourner/venir-et-se-deplacer-a-lyon/transport-et-transfert/parking-des-celestins) et [avis Michelin](https://nl.viamichelin.be/web/Toeristische-Bezienswaardigheid/Lyon-69002-Parking_des_Celestins-2604635b185d)
- [Parking Ravet, Chambéry, dossier de presse](https://www.v2com-newswire.com/fr/salle-de-presse/post_files/4185/download)
- [Silo du campus Paris-Saclay](https://epa-paris-saclay.fr/wp-content/uploads/2020/07/2020-07-27-Livraison-parking-silo-Campus-Paris-Saclay.pdf)
- [Silo de l'îlot Académie, Angers, notice](https://www.angers.fr/fileadmin/user_upload/a_-_notice_explicative.pdf)
- [Parking Sulzer, Nice, fiche PSS](https://www.pss-archi.eu/immeubles/FR-06088-33945.html) et [Nicolaï ingénierie](https://nicolai-ingenierie-beton.com/projet/parking-sulzer/)
- [Silo de la ZAC Georges Besse 2, Nîmes](https://www.objectifgard.com/actualites/nimes-avec-tisspark-le-parc-georges-besse-prend-sa-place-8471.php)
