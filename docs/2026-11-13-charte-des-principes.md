# Parkimo : charte des principes

Version 1 du 13 novembre 2026. Elle intègre les sept décisions de l'utilisateur (section 9). Elle décrit le système visé, pas l'état du code : le prototype du squelette applique déjà une partie de ces principes, mais chacun y est écrit à part.

## À quoi sert cette charte

Le plan n'est plus réglé par des cas particuliers mais par des principes. Chaque principe est écrit ici, porte une mesure, et s'applique à tous les plans. Devant un défaut, on ne corrige pas le cas : on cherche le principe qui manque ou qui est faux, on le corrige ici, puis dans le code.

## 1. Tout est une emprise

Chaque élément déclare trois choses et rien d'autre : ce qu'il **occupe**, ce qu'il **réserve** autour de lui, et sa **nature**.

| Élément | Occupe | Réserve | Nature |
|---|---|---|---|
| Voie | sa chaussée | rien | circulation voitures |
| Carrefour | la surface entre les voies qui se rencontrent | rien | circulation voitures |
| Rampe perpendiculaire à une voie | sa chaussée : c'est une voie du squelette, sans stationnement, branchée directement | rien | circulation voitures |
| Rampe parallèle à une voie, dans une rangée | sa voie de 3,25 m | ses deux épures de giration (entrée et sortie en S) | circulation voitures |
| Bande piétonne | 0,80 m dans la largeur de la voie | rien | circulation piétons |
| Passage piéton | 0,80 m en travers d'une voie, d'équerre | rien | circulation piétons |
| Rangée | la bande entre la voie et son fond | rien | stationnement |
| Place | son quadrilatère dans la rangée | rien | stationnement |
| Poteau, voile | sa section, sur une ligne de place ou un fond de rangée | rien | structure |
| Travée pleine | sa ligne entre deux poteaux | rien | structure |
| Circulation verticale | à définir | à définir | circulation piétons |

## 2. Qui cède : l'ordre de priorité

Quand deux emprises incompatibles se rencontrent, la moins prioritaire cède : elle est raccourcie, déplacée ou retirée.

1. Circulation des voitures (chaussées, carrefours, rampes, épures)
2. Circulation des piétons (bandes, passages, circulations verticales)
3. Stationnement (rangées, places)
4. Structure (poteaux, voiles, travées pleines)

Les places remplissent tout ce qui reste après les circulations : c'est la règle du maximum de places. La structure cède aux places : un poteau ne condamne jamais une place, il se déplace sur une autre ligne.

## 3. Ce qui peut se superposer

| | Voitures | Piétons | Places | Structure |
|---|---|---|---|---|
| **Voitures** | oui (carrefour) | oui, marqué (bande, passage) ; de préférence hors des épures de giration | non | non |
| **Piétons** | | oui | non | non |
| **Places** | | | non | au bord seulement |
| **Structure** | | | | oui (poteau sur travée) |

« Au bord seulement » : la structure tient sur une ligne de place ou sur le fond d'une rangée, jamais dans une place.

## 4. Le plan se construit par couches

Dans cet ordre, chaque couche ne lisant que les précédentes :

1. **Squelette** : nœuds et voies, donnés par l'utilisateur. Une rampe perpendiculaire en fait partie.
2. **Chaussées et carrefours**, déduits du squelette.
3. **Objets posés** : rampes en rangée, plus tard circulations verticales et locaux, avec leurs réserves.
4. **Cheminements piétons** : bandes et passages.
5. **Trame commune** : les places et la structure se règlent ensemble (section 5).
6. **Contrôles** : les principes de la section 8.

L'utilisateur agit sur les couches 1 et 3, et peut reprendre la main sur 4 et 5. Le reste est déduit.

**Niveaux** : tous les niveaux partagent le même squelette. Structure et distribution se superposent d'un niveau à l'autre ; seuls les objets posés et l'usage des rangées changent (rez-de-chaussée, étage courant, dernier niveau).

## 5. La trame commune : places et structure

La structure n'est pas une règle fixe : elle dépend du système porteur, choisi pour l'ouvrage.

| Système porteur | Appuis | Portée |
|---|---|---|
| Poteaux et poutres | une file toutes les deux ou trois places | à fixer |
| Voiles béton | un voile toutes les deux ou trois places | à fixer |
| Grande portée | aucun appui intermédiaire | jusqu'à 16 m, d'après l'utilisateur (parkings enterrés, exemple cité : le Pré aux Pêcheurs à Antibes) |

La largeur de place se choisit avec la trame, dans cet ordre de préférence :

1. **2,50 m** par défaut.
2. **2,40 m** si la structure y gagne.
3. **2,30 m** à la marge, seulement pour gagner une place.

Le logiciel choisit, rangée par rangée, le nombre de places par travée et la largeur qui servent le mieux les objectifs de la section 7.

## 6. Une seule règle de raccord

Autour d'un nœud, entre deux voies voisines, les lignes de même rang se prolongent jusqu'à leur rencontre : bord de chaussée, bord de bande piétonne, fond de rangée. Cette règle donne les carrefours, les angles du bâtiment, les raccords de bandes. Quand la rencontre est trop loin (angle très aigu, voie trop courte), les deux voies sont coupées d'équerre et une cale ferme l'intervalle.

Dans un angle rentrant bordé de places des deux côtés, la rangée de la voie la plus longue garde le coin.

## 7. Les objectifs, dans l'ordre

Quand le logiciel a un choix à faire, il le fait pour l'ensemble du plan, en comparant dans cet ordre :

1. **Conformité** : aucun principe de la section 8 violé.
2. **Continuité des circulations** : réseau piéton d'un seul tenant ; le moins de traversées possible ; cheminements hors des épures de giration ; bandes côté façade extérieure.
3. **Nombre de places.**
4. **Régularité de la structure** : files alignées, portées égales, contreventement dans deux sens.

## 8. Chaque principe est un contrôle

Les contrôles sont rejoués sur des plans déformés au hasard, pas sur des cas choisis.

| Principe | Mesure | État dans le prototype |
|---|---|---|
| Les surfaces ne se recouvrent pas et ne laissent pas de vide | somme des aires / emprise = 1 | mesuré : 1,000 sur l'exemple, 1,007 avec un carrefour en biais ; violé sur 36 des 60 plans déformés au hasard, cause à établir au lot 2 |
| Aucun poteau dans une circulation de voitures | nombre de poteaux en voie ou dans une épure = 0 | mesuré : 0 sur 60 déformations |
| Aucun poteau dans une place | nombre de poteaux dans une place = 0 | mesuré : 0 sur 60 déformations ; un poteau lâché à la main dans une place gagne la ligne la plus proche |
| Le réseau piéton est d'un seul tenant | nombre de tronçons = 1 | mesuré : 60 sur 60 |
| Un passage piéton est d'équerre sur la voie traversée | angle = 90° | appliqué, non mesuré |
| Un cheminement ne traverse pas une épure de giration | bandes dans une épure = 0, de préférence | appliqué et mesuré : la bande bascule de l'autre côté de la voie |
| Toute place est accessible depuis une voie | places sans accès = 0 | non mesuré |
| Toute rampe est circulable | épure libre de façade, de place et d'autre rampe | contrôlé contre les poteaux seulement |
| Toute voie est reliée à une entrée | voies isolées = 0 | non mesuré |
| La structure est contreventée | travées pleines dans deux sens | mesuré, sans calcul ; violé dès que le plan est déformé, les travées pleines ne suivant pas |
| Portée maximale respectée | plus longue portée ≤ limite du système porteur | affichée, limites à fixer |

## 9. Décisions de l'utilisateur, 13 novembre 2026

1. **Poteau et place** : un poteau ne condamne pas une place, il se déplace sur une autre ligne.
2. **Piétons et girations** : de préférence, la circulation piétonne bascule hors des épures de giration.
3. **Objectifs** : la continuité des circulations passe avant le nombre de places.
4. **Trame** : pas de règle fixe à trois places. En béton, souvent deux ou trois places entre voiles ; cela varie avec la structure et la portance ; des dalles peuvent porter 16 m sans poteau.
5. **Largeur de place** : 2,50 m de préférence ; 2,40 m si la structure y gagne ; 2,30 m à la marge, pour gagner une place.
6. **Niveaux** : même squelette ; structures et distributions se superposent.
7. **Rampes** : l'entrée en S n'est pas le principe général. Une rampe à 90° d'une voie se branche directement dessus, sans S.

## 10. Ce qui reste à fixer

1. Section d'un poteau et épaisseur d'un voile.
2. Portée maximale de chaque système porteur.
3. Hauteur d'étage et pente de rampe par défaut (aujourd'hui 2,80 m et 15 %).
4. Largeur d'une rampe perpendiculaire à double sens, et ce que « coupler » veut dire pour elle.
5. Emprise et réserve d'une circulation verticale.

## 11. Les valeurs sont des données

Les dimensions ne sont pas dans le code mais dans un tableau, avec leur source. Valeurs en place aujourd'hui :

| Valeur | Aujourd'hui | Source |
|---|---|---|
| Places à 90° : largeur, profondeur, allée | 2,50 m ; 5,00 m ; 5,00 m | décision 5 ; allée NF P 91-100 tab. 2 |
| Places à 60° : largeur, profondeur, allée | 2,25 m ; 5,15 m ; 4,00 m | NF P 91-100 tab. 2 |
| Places à 45° : largeur, profondeur, allée | 2,20 m ; 4,80 m ; 3,50 m | NF P 91-100 tab. 2 |
| Places en long : longueur, largeur, allée | 5,00 m ; 2,50 m ; 3,50 m | NF P 91-100 tab. 2 |
| Voie sans stationnement | 5,50 m | NF P 91-100 § 3.3 (double sens) |
| Bande piétonne | 0,80 m, dans la voie | règle de l'utilisateur |
| Voie de rampe | 3,25 m | Neufert p. 274-276 (sens unique) |
| Giration : rayon à l'axe, largeur balayée | 6,50 m ; 2 × 2,00 m | moteur Parkimo (`giration.rs`) |
| Rampe droite | 20 m (2,80 m à 15 %) | Neufert, arrondi au pas |
| Trame de structure | deux ou trois places par travée | décision 4 |
| Pas d'accroche des nœuds | 2,50 m, angles de 15° | prototype |

## 12. Reconstruction, lot par lot

Chaque lot est livré et vérifié seul ; le prototype reste utilisable entre deux lots.

1. **Registre des principes** (fait le 13 novembre, sept principes) : les contrôles de la section 8 deviennent un tableau dans le code, affiché au bilan (principes respectés, principes violés) et rejoué sur les plans au hasard.
2. **Emprises et arbitrage** : une seule fonction fait céder les emprises selon l'ordre de la section 2, à la place des retraits écrits à la main.
3. **Trame commune** : places et structure réglées ensemble, systèmes porteurs, largeurs 2,50 / 2,40 / 2,30.
4. **Rampes** : la rampe perpendiculaire devient une voie du squelette ; la rampe en rangée garde ses épures ; les cheminements les évitent.
5. **Niveaux** sur un même squelette.

## 13. Méthode de travail

1. Un défaut est constaté sur un plan.
2. On le rattache à un principe de cette charte : violé, mal écrit, ou manquant.
3. On corrige la charte, puis la règle dans le code, et on ajoute sa mesure aux contrôles.
4. Les contrôles sont rejoués sur les plans au hasard : la correction vaut pour tous.
