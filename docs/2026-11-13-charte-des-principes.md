# Parkimo : charte des principes

Version 0 du 13 novembre 2026, à corriger. Elle décrit le système visé, pas l'état du code : le prototype du squelette applique déjà une partie de ces principes, mais chacun y est écrit à part.

## À quoi sert cette charte

Le plan n'est plus réglé par des cas particuliers mais par des principes. Chaque principe est écrit ici, porte une mesure, et s'applique à tous les plans. Devant un défaut, on ne corrige pas le cas : on cherche le principe qui manque ou qui est faux, on le corrige ici, puis dans le code.

## 1. Tout est une emprise

Chaque élément déclare trois choses et rien d'autre : ce qu'il **occupe**, ce qu'il **réserve** autour de lui, et sa **nature**.

| Élément | Occupe | Réserve | Nature |
|---|---|---|---|
| Voie | sa chaussée | rien | circulation voitures |
| Carrefour | la surface entre les voies qui se rencontrent | rien | circulation voitures |
| Rampe | sa voie de 3,25 m | ses deux épures de giration | circulation voitures |
| Bande piétonne | 0,80 m dans la largeur de la voie | rien | circulation piétons |
| Passage piéton | 0,80 m en travers d'une voie | rien | circulation piétons |
| Rangée | la bande entre la voie et son fond | rien | stationnement |
| Place | son quadrilatère dans la rangée | rien | stationnement |
| Poteau | son point (section à définir) | rien | structure |
| Travée pleine | sa ligne entre deux poteaux | rien | structure |
| Circulation verticale | à définir | à définir | circulation piétons |

## 2. Qui cède : l'ordre de priorité

Quand deux emprises incompatibles se rencontrent, la moins prioritaire cède : elle est raccourcie, déplacée ou retirée.

1. Circulation des voitures (chaussées, carrefours, voies de rampe, épures)
2. Circulation des piétons (bandes, passages, circulations verticales)
3. Structure (poteaux, travées pleines)
4. Stationnement (rangées, places)

Les places sont en dernier : elles remplissent tout ce qui reste. C'est la règle du maximum de places.

## 3. Ce qui peut se superposer

| | Voitures | Piétons | Structure | Places |
|---|---|---|---|---|
| **Voitures** | oui (carrefour) | oui, marqué (bande, passage) | non | non |
| **Piétons** | | oui | non | non |
| **Structure** | | | oui (poteau sur travée) | au bord seulement |
| **Places** | | | | non |

« Au bord seulement » : un poteau tient sur une ligne de place ou sur le fond d'une rangée, jamais dans une place.

## 4. Le plan se construit par couches

Dans cet ordre, chaque couche ne lisant que les précédentes :

1. **Squelette** : nœuds et voies, donnés par l'utilisateur.
2. **Chaussées et carrefours**, déduits du squelette.
3. **Objets posés** : rampes, plus tard circulations verticales et locaux, avec leurs réserves.
4. **Cheminements piétons** : bandes et passages.
5. **Rangées et places**, dans ce qui reste.
6. **Structure**, sur la trame des places qui restent.
7. **Contrôles** : les principes de la section 7.

L'utilisateur agit sur les couches 1 et 3, et peut reprendre la main sur 4 et 6. Le reste est déduit.

## 5. Une seule règle de raccord

Autour d'un nœud, entre deux voies voisines, les lignes de même rang se prolongent jusqu'à leur rencontre : bord de chaussée, bord de bande piétonne, fond de rangée. Cette règle donne les carrefours, les angles du bâtiment, les raccords de bandes. Quand la rencontre est trop loin (angle très aigu, voie trop courte), les deux voies sont coupées d'équerre et une cale ferme l'intervalle.

Dans un angle rentrant bordé de places des deux côtés, la rangée de la voie la plus longue garde le coin.

## 6. Les objectifs, dans l'ordre

Quand le logiciel a un choix à faire, il le fait pour l'ensemble du plan, en comparant dans cet ordre :

1. **Conformité** : aucun principe de la section 7 violé.
2. **Continuité des circulations** : réseau piéton d'un seul tenant, le moins de traversées possible, bandes côté façade extérieure.
3. **Nombre de places.**
4. **Régularité de la structure** : files alignées, portées égales, contreventement dans deux sens.

## 7. Chaque principe est un contrôle

Les contrôles sont rejoués sur des plans déformés au hasard, pas sur des cas choisis.

| Principe | Mesure | État dans le prototype |
|---|---|---|
| Les surfaces ne se recouvrent pas et ne laissent pas de vide | somme des aires / emprise = 1 | mesuré : 1,000 sur l'exemple, 1,007 avec un carrefour en biais |
| Aucun poteau dans une circulation de voitures | nombre de poteaux en voie ou dans une épure = 0 | mesuré : 0 sur 60 déformations |
| Aucun poteau dans une place | nombre de places condamnées = 0 | mesuré : 0 sur 60 déformations |
| Le réseau piéton est d'un seul tenant | nombre de tronçons = 1 | mesuré : 60 sur 60 |
| Un passage piéton est d'équerre sur la voie traversée | angle = 90° | appliqué, non mesuré |
| Toute place est accessible depuis une voie | places sans accès = 0 | non mesuré |
| Toute rampe est circulable | épure libre de façade, de place et d'autre rampe | contrôlé contre les poteaux seulement |
| Toute voie est reliée à une entrée | voies isolées = 0 | non mesuré |
| La structure est contreventée | travées pleines dans deux sens | mesuré, sans calcul |
| Portée maximale respectée | plus longue poutre ≤ limite | affichée, limite à fixer |

## 8. Les valeurs sont des données

Les dimensions ne sont pas dans le code mais dans un tableau, avec leur source. Valeurs en place aujourd'hui :

| Valeur | Aujourd'hui | Source |
|---|---|---|
| Places à 90° : pas, profondeur, allée | 2,50 m ; 5,00 m ; 5,00 m | prototype ; allée NF P 91-100 tab. 2 |
| Places à 60° : largeur, profondeur, allée | 2,25 m ; 5,15 m ; 4,00 m | NF P 91-100 tab. 2 |
| Places à 45° : largeur, profondeur, allée | 2,20 m ; 4,80 m ; 3,50 m | NF P 91-100 tab. 2 |
| Places en long : longueur, largeur, allée | 5,00 m ; 2,50 m ; 3,50 m | NF P 91-100 tab. 2 |
| Voie sans stationnement | 5,50 m | NF P 91-100 § 3.3 (double sens) |
| Bande piétonne | 0,80 m, dans la voie | règle de l'utilisateur |
| Voie de rampe | 3,25 m | Neufert p. 274-276 (sens unique) |
| Giration : rayon à l'axe, largeur balayée | 6,50 m ; 2 × 2,00 m | moteur Parkimo (`giration.rs`) |
| Rampe droite | 20 m (2,80 m à 15 %) | Neufert, arrondi au pas |
| Trame de structure | une file toutes les trois places | règle de l'utilisateur |
| Pas d'accroche des nœuds | 2,50 m, angles de 15° | prototype |

## 9. Questions à trancher

1. **Structure ou places** : la charte met la structure avant les places. Un poteau peut-il condamner une place, ou doit-il toujours se déplacer sur une ligne ?
2. **Piétons et épures** : un cheminement peut-il traverser l'épure de giration d'une rampe, avec un passage marqué, ou doit-il l'éviter ?
3. **Ordre des objectifs** : la continuité des circulations passe-t-elle bien avant le nombre de places ?
4. **Trame** : trois places par travée est-il une règle fixe, ou une fourchette (deux à quatre) dans laquelle le logiciel choisit ? Quelle portée maximale ?
5. **Largeur de place** : le prototype dessine des places de 2,50 m ; la NF P 91-100 admet 2,30 m à 90°. Laquelle retenir par défaut ?
6. **Niveaux** : rez-de-chaussée, étage courant et dernier niveau partagent-ils le même squelette, avec des objets posés différents ?
7. **Accès des rampes** : l'entrée en S depuis l'allée voisine est-elle le principe général, les autres typologies venant ensuite ?

## 10. Méthode de travail

1. Un défaut est constaté sur un plan.
2. On le rattache à un principe de cette charte : violé, mal écrit, ou manquant.
3. On corrige la charte, puis la règle dans le code, et on ajoute sa mesure aux contrôles.
4. Les contrôles sont rejoués sur les plans au hasard : la correction vaut pour tous.
