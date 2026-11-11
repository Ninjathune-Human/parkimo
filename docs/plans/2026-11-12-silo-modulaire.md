# Silo modulaire : plan technique

12 novembre 2026. Objectif fixé : un parc en ouvrage entièrement modulaire. Les rampes se posent où l'on veut et leurs raccords au plateau se font seuls ; le silo épouse une emprise quelconque (triangle, pan coupé, L).

## 1. Où en est le moteur

- `src/ouvrage/plateau.rs` (2 550 lignes) construit un plateau **rectangulaire** dans un repère local : modules en travers (une allée et deux rangées), rangées de longueur fixe, deux bouts identiques. `chercher` trouve le plus grand rectangle inscrit dans la zone constructible.
- Les rampes sont onze **implantations écrites à la main**, chacune avec sa géométrie et ses épures de giration tracées par arcs et droites (`Trace`). Elles ne vivent qu'aux deux bouts du rectangle. Depuis hier, la rampe « en rangée » se glisse le long de sa rangée : c'est un cas particulier, pas une solution générale.
- À côté, le moteur de surface sait déjà remplir un **polygone quelconque** : rangées découpées par la limite (`rows.rs`, `strip_intervals`), réseau de voies et sens de circulation (`circulation.rs`). Le simulateur de giration (`src/giration/`) sait chercher une trajectoire réelle entre deux points (A* hybride).

Conclusion : le silo est rigide parce que sa forme, ses rampes et ses raccords sont un seul bloc de code. Il faut les séparer.

## 2. Principe : quatre couches indépendantes

1. **Emprise** : un polygone, par niveau. Le rectangle devient un cas particulier.
2. **Trame** : une direction d'allées et un pas de structure (module en travers, travée en long), posés sur l'emprise. Elle donne les files de poteaux et les rangées possibles.
3. **Objets posés** : rampes, escaliers et ascenseurs, contrôle d'accès, locaux. Chaque objet a une emprise, une zone à garder libre et des **bouches** : un point, un cap, une largeur, et le niveau qu'elle dessert.
4. **Remplissage** : les rangées de places occupent ce qui reste, découpées par l'emprise et par les objets ; puis le réseau d'allées est calculé, et chaque bouche y est raccordée.

Une rampe n'est plus une implantation : c'est un objet du catalogue (droite, en U, hélice simple ou double) que l'on pose, tourne et déplace. Le moteur ne sait plus où elle « doit » être ; il sait la raccorder.

## 3. Raccords automatiques

Pour chaque bouche, le moteur cherche l'axe d'allée le plus proche qui roule dans le bon sens, puis trace le raccord :

1. d'abord par une forme simple calculée (tout droit, quart de tour, S), comme aujourd'hui ;
2. sinon par le simulateur de giration, dans l'espace laissé libre.

La bande balayée par le raccord est retirée des places et gardée libre de poteaux. Si aucun raccord n'existe, l'objet reste posé et un constat le dit (« rampe non raccordée »), avec la raison. Rien n'est refusé en silence.

## 4. Forme libre

- Les rangées suivent la trame et sont coupées par le polygone : chaque rangée a sa longueur. Un triangle donne des rangées de plus en plus courtes ; un pan coupé raccourcit celles du coin.
- Les allées se ferment par une allée de bout qui longe la limite, ou en impasse courte quand la norme le permet (à contrôler : demi-tour, longueur).
- Les poteaux restent sur la trame ; en rive, ils se posent à l'intersection de la trame et de la façade.
- La recherche d'emprise ne cherche plus un rectangle : elle prend la zone constructible entière, ou le polygone dessiné.
- Première étape volontairement limitée : **une seule direction de trame** par silo. Deux directions (un L dont chaque aile a sa trame) viendront ensuite, par zones.

## 5. Niveaux

Chaque niveau est calculé avec les bouches qui y sont actives : bas des rampes au rez, bas et haut aux étages, haut seul au dernier. Les trémies sont l'emprise des rampes dans la dalle du dessus. L'affichage RDC, étage courant, dernier niveau existe déjà et ne change pas.

Demi-niveaux et plateaux inclinés ne sont pas des objets mais des systèmes de plateau : ils restent liés au rectangle dans un premier temps.

## 6. Lots

| Lot | Contenu | Vérification |
| --- | --- | --- |
| A. Emprise polygonale | Plateau sur un polygone, une trame, rangées coupées, allées de bout, poteaux. Pan coupé et triangle. Sans rampe d'abord | Sur un rectangle : même nombre de places qu'aujourd'hui, à 1 % près, sur les 5 scénarios et les 3 densités. Sur triangle et pan coupé : aucune place hors emprise, toute place sur une allée, aucun poteau dans une allée |
| B. Rampes en objets | Catalogue (droite, U, hélices), emprise, zone libre, bouches. Pose sur la trame, quatre orientations | Les onze implantations actuelles redonnées par des objets posés, mêmes places à 1 % près |
| C. Raccords | Formes simples puis simulateur ; bande balayée ; constat si impossible | Chaque bouche raccordée sur les scénarios ; une rampe posée au milieu ou en coin se raccorde ; un cas impossible rend son constat |
| D. Niveaux et trémies | Plan par niveau selon les bouches actives, dalles ouvertes | Décompte par niveau égal à la somme des plans ; scène 3D cohérente |
| E. Interface | Dessin de l'emprise du silo (sommets, pan coupé), glisser et tourner les objets, aperçu pendant le geste, commandes pour l'assistant | Essais dans Chrome, PC et iPhone ; annuler et rétablir |
| F. Suites | Escaliers, contrôle d'accès et local posés comme objets ; exports DXF et IFC ; deux directions de trame | Essais d'export ; non-régression |

L'ancien générateur rectangulaire reste en service tant que le nouveau ne l'égale pas sur les scénarios de référence. La bascule se fait lot par lot, derrière un réglage, sans jamais retirer une fonction publiée.

## 7. Risques

- **Rendement.** Un remplissage général place souvent un peu moins bien qu'un générateur écrit pour le rectangle. Garde-fou : le seuil de 1 % du lot A.
- **Temps de calcul.** Le simulateur de giration est lent (jusqu'à quelques secondes). Il ne sert qu'en second recours et tourne en arrière-plan ; pendant un glisser, seul l'objet bouge, le plateau est recalculé au relâchement.
- **Structure.** Sur une forme libre, les portées de rive deviennent irrégulières. Le moteur les signale ; il ne les calcule pas.
- **Évacuation.** Les distances aux escaliers se mesurent le long des allées, plus en ligne droite dans un rectangle : à reprendre au lot F.

## 8. Trois choix qui vous reviennent

1. Orientation des rampes : quatre directions alignées sur la trame, ou angle libre ?
2. Ordre : la forme libre d'abord (lot A), ou les rampes libres d'abord (lots B et C sur le rectangle actuel) ?
3. Formes en L : une seule trame suffit-elle pour commencer, ou faut-il deux directions dès le départ ?
