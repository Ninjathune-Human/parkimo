# Parkimo : charte des principes

Version 2 du 13 novembre 2026. Elle intègre les sept décisions de l'utilisateur (section 9) et les valeurs de structure relevées chez les constructeurs et dans les guides spécialisés (sections 5 et 14). Elle décrit le système visé, pas l'état du code : le prototype du squelette applique déjà une partie de ces principes, mais chacun y est écrit à part.

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

| Système porteur | Appuis le long de la rangée | Portée en travers de la rangée | Section retenue |
|---|---|---|---|
| Poteaux et poutres, béton | une file toutes les deux ou trois places : 5,00 ou 7,50 m entre nus | 7,50 m courant, 10 m au plus | poteau 0,40 × 0,40 m (0,30 m au moins) |
| Voiles béton | un voile toutes les deux ou trois places : 5,00 ou 7,50 m entre nus | dalle portant de voile à voile : 5,00 à 7,50 m | voile 0,20 m (usuel 0,15 à 0,20 m, jamais moins de 0,12 m) |
| Grande portée, acier et béton collaborant | une file par place (2,50 m) ou toutes les deux places (5,00 m), en façade ou en fond de rangée | 16,00 m courant (deux places de 5,00 m et une allée de 6,00 m), 16,50 m au plus | poteau en profilé, environ 0,20 m (HEA 180 relevé pour quatre niveaux) |
| Grande portée, béton précontraint | une file toutes les deux ou trois places (5,00 ou 7,50 m), en façade ou en fond de rangée | 16,00 m courant, 16,50 m au plus | poteau 0,40 × 0,40 m ; plancher de 0,40 m sans retombée |

Trois règles accompagnent ce tableau :

1. **La trame se compte entre nus.** L'entraxe vaut le nombre de places multiplié par leur largeur, plus la section de l'appui. Un entraxe de 7,20 m avec des poteaux de 0,40 m ne laisse que 6,80 m pour trois places.
2. **Une rangée à 90° de la norme tient sous une grande portée** : 5,00 + 5,00 + 5,00 = 15,00 m de mur à mur (NF P 91-100), pour 16,00 m de portée courante. Aucun poteau dans le plateau.
3. **Tenue au feu** : structure R 90 et planchers REI 90 pour un parc de plus de deux niveaux jusqu'à 28 m ; une dalle pleine sans poutre fait alors 0,20 m au moins. Pour un parc largement ventilé en acier, ArcelorMittal annonce une justification particulière au feu, sans protection rapportée : à vérifier dans l'arrêté du 9 mai 2006.

Les sources sont en section 14. Ce sont des valeurs d'esquisse : le bureau d'études structure les confirme projet par projet.

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
| Deux voies ne se croisent qu'à un nœud | chaussées de voies sans nœud commun qui se recouvrent = 0 | mesuré ; alerte de conception : 20 des 60 plans déformés au hasard |
| Deux rangées ne se recouvrent pas | rangées de voies différentes qui se recouvrent = 0 | mesuré ; alerte de conception : 51 des 60 plans déformés au hasard (îlots trop étroits). L'arbitrage tranche : la voie la plus longue garde ses places |
| Une emprise posée n'en recouvre aucune qu'elle exclut | garanti par l'arbitrage unique | appliqué aux places et aux poteaux de la trame automatique |
| Aucun poteau dans une circulation de voitures | nombre de poteaux en voie ou dans une épure = 0 | mesuré : 0 sur 60 déformations |
| Aucun poteau dans une place | nombre de poteaux dans une place = 0 | mesuré : 0 sur 60 déformations ; un poteau lâché à la main dans une place gagne la ligne la plus proche ; s'il s'y retrouve quand même, la place reste et c'est le poteau qui est signalé |
| Le réseau piéton est d'un seul tenant | nombre de tronçons = 1 | mesuré : 60 sur 60 |
| Un passage piéton est d'équerre sur la voie traversée | angle = 90° | appliqué, non mesuré |
| Un cheminement ne traverse pas une épure de giration | bandes dans une épure = 0, de préférence | appliqué et mesuré : la bande bascule de l'autre côté de la voie |
| Toute place est accessible depuis une voie | places sans accès = 0 | non mesuré |
| Toute rampe est circulable | épure libre de façade, de place et d'autre rampe | contrôlé contre les poteaux seulement |
| Toute voie est reliée à une entrée | voies isolées = 0 | non mesuré |
| La structure est contreventée | travées pleines dans deux sens | mesuré, sans calcul ; violé dès que le plan est déformé, les travées pleines ne suivant pas |
| Portée maximale respectée | plus longue portée ≤ limite du système porteur | affichée ; limites fixées en section 5, contrôle à brancher au lot 3 |

## 9. Décisions de l'utilisateur, 13 novembre 2026

1. **Poteau et place** : un poteau ne condamne pas une place, il se déplace sur une autre ligne.
2. **Piétons et girations** : de préférence, la circulation piétonne bascule hors des épures de giration.
3. **Objectifs** : la continuité des circulations passe avant le nombre de places.
4. **Trame** : pas de règle fixe à trois places. En béton, souvent deux ou trois places entre voiles ; cela varie avec la structure et la portance ; des dalles peuvent porter 16 m sans poteau.
5. **Largeur de place** : 2,50 m de préférence ; 2,40 m si la structure y gagne ; 2,30 m à la marge, pour gagner une place.
6. **Niveaux** : même squelette ; structures et distributions se superposent.
7. **Rampes** : l'entrée en S n'est pas le principe général. Une rampe à 90° d'une voie se branche directement dessus, sans S.

## 10. Ce qui reste à fixer

1. Largeur d'une rampe perpendiculaire à double sens, et ce que « coupler » veut dire pour elle.
2. Emprise et réserve d'une circulation verticale.
3. Section d'un poteau béton dans un parc français : la valeur de 0,40 m vient de guides britanniques, faute de source française chiffrée. À confirmer.
4. Dimensions minimales au feu d'un poteau et d'un voile (NF EN 1992-1-2, valeurs tabulées) : non relevées en ligne, à lire dans la norme.
5. Structure du parking du Pré aux Pêcheurs à Antibes : aucune source publique trouvée ; la portée de 16 m y reste une indication de l'utilisateur.

Fixé depuis la version 1 : section des appuis et portées (section 5) ; hauteur d'étage de 2,80 m et pente de 15 % (section 11).

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
| Trame de structure | deux ou trois places par travée, comptées entre nus | décision 4 ; section 5 |
| Portée en travers : poteaux et poutres béton | 7,50 m, 10 m au plus | section 14, sources 13 à 15 |
| Portée en travers : grande portée | 16,00 m, 16,50 m au plus | section 14, sources 1 à 12 |
| Poteau béton | 0,40 × 0,40 m | section 14, source 13 ; à confirmer |
| Poteau acier | 0,20 m | section 14, source 4 |
| Voile béton | 0,20 m | section 14, source 16 |
| Hauteur d'étage | 2,80 m (2,10 m libres et 0,70 m de plancher en acier de 16 m) | section 14, source 2 ; NF P 91-100 § 3.2 demande 2,20 m sous plafond et 2,00 m sous poutre |
| Pente de rampe | 15 % | Neufert p. 274 ; section 14, source 2 (15 % couverte, 10 % à ciel ouvert) ; plafond NF P 91-100 : 18 % |
| Pas d'accroche des nœuds | 2,50 m, angles de 15° | prototype |

## 12. Reconstruction, lot par lot

Chaque lot est livré et vérifié seul ; le prototype reste utilisable entre deux lots.

1. **Registre des principes** (fait le 13 novembre, sept principes) : les contrôles de la section 8 deviennent un tableau dans le code, affiché au bilan (principes respectés, principes violés) et rejoué sur les plans au hasard.
2. **Emprises et arbitrage** (fait le 13 novembre) : une seule règle fait céder les emprises selon l'ordre de la section 2, à la place des retraits écrits à la main. Le principe des surfaces, trop global, est remplacé par deux principes précis : voies qui se croisent sans nœud, rangées qui se recouvrent. Les 36 plans au hasard en défaut s'expliquent par ces deux cas, qui sont des alertes de conception et non des défauts du logiciel.
3. **Trame commune** : places et structure réglées ensemble, systèmes porteurs et portées de la section 5, largeurs 2,50 / 2,40 / 2,30.
4. **Rampes** : la rampe perpendiculaire devient une voie du squelette ; la rampe en rangée garde ses épures ; les cheminements les évitent.
5. **Niveaux** sur un même squelette.

## 13. Méthode de travail

1. Un défaut est constaté sur un plan.
2. On le rattache à un principe de cette charte : violé, mal écrit, ou manquant.
3. On corrige la charte, puis la règle dans le code, et on ajoute sa mesure aux contrôles.
4. Les contrôles sont rejoués sur les plans au hasard : la correction vaut pour tous.

## 14. Sources des valeurs de structure

Relevé du 13 novembre 2026 : constructeurs, industriels, organismes professionnels, réalisations publiées. Chaque ligne dit ce que la source établit.

**Grande portée, acier et béton collaborant**

1. ArcelorMittal, journées GC'2017 de l'AFGC, « Parcs de stationnement largement ventilés en acier » : solives de 15 à 16 m, deux rangées de 5 m, entraxe de deux à quatre voitures, plancher de 140 mm, portée libre de 16 m au-dessus de deux places et de l'allée. https://afgc.asso.fr/app/uploads/2017/04/3-4-AFGC_2017_mutualisation-du-stationnement.pdf
2. bauforumstahl, guide « Parkhäuser in Stahl- und Verbundbauweise » : poutre mixte d'environ 16 m sans appui, standard des parcs aériens ; plancher de 70 cm ; étage de 2,80 m ; 2,10 m libres ; rampe à 15 % couverte et 10 % à ciel ouvert.
3. Goldbeck, brochure « Multi-storey Car Parks » : module de 16,00 m (deux places de 5,00 m, allée de 6,00 m), poutres de 16,00 m tous les 2,50 m, poteaux en façade, place de 2,50 m. https://issuu.com/goldbeckgmbh/docs/gb_pros_ph_eng_l03_einzelseiten
4. Parking du Centre hospitalier de Luxembourg, 643 places sur quatre niveaux (Infosteel) : poteaux HEA 180 et poutres IPE 400 tous les 2,5 m, deux travées de 16,5 m, dalle de 13 cm.
5. Parking des Machines à Nantes (ArcelorMittal Constructalia) : portée de 16 m, trame de 5,2 m, solives tous les 2,6 m, poteaux HEA. https://constructalia.arcelormittal.com/en/case_study_gallery/france/multi_storey_car_park_les_machines_in_nantes_with_arcelormittal_steel_structure
6. Parkings des Argoulets et de Balma-Gramont à Toulouse (ArcelorMittal Constructalia) : trame de 15 × 10 m, solives de 15 m tous les 3,3 m.
7. Parking Enovos à Esch-sur-Alzette (Infosteel) : trame de 15,75 × 5 m, 2,20 m libres sous poutres.
8. Max Bögl, systèmes de parkings : acier et béton collaborant, une file tous les 2,50 m, aucun poteau intérieur ; béton préfabriqué, une file tous les 5,00 ou 7,50 m, aucun poteau intérieur.

**Grande portée, béton**

9. Parking silo République à Reims, 745 places, 2020 (Batiweb, Batiproduits) : planchers précontraints allégés de 16 m sans poteau intermédiaire ni retombée, 40 cm d'épaisseur. https://www.batiweb.com/actualites/publi-redactionnels/la-solution-technique-planchers-post-contraint-alleges-u-boot-beton-pour-la-realisation-de-parking-silo-sur-la-ville-de-reims-39803
10. Concast (préfabricant, Irlande) : dalles alvéolées jusqu'à 16,2 m ; trame type de 16,5 × 7,5 m, trois places par travée, dalles de 400 mm. British Precast : portées libres jusqu'à 16 m.
11. Parc relais Mermoz-Pinel à Bron, souterrain, quatre niveaux (Gautier + Conquet) : poteaux sur une trame d'environ 15 × 5 m.
12. Dalles alvéolées précontraintes (Batiproduits, fiche fabricant) : hauteurs de 15 à 40 cm, portée maximale de 15 à 20 m. ByBéton : une vingtaine de mètres en pré-tension, davantage en post-tension.

**Poteaux et poutres, voiles**

13. Guides britanniques de conception des parcs (« Steel-framed car parks » et note « Car park design », consultés en ligne) : trame usuelle de 7,2 m pour trois places de 2,4 m ; poteaux réels de 400 × 400 mm, qui ramènent le libre à 6,8 m ; compter 0,30 m de dégagement et une demi-section de chaque côté.
14. Kings Dock, Liverpool (New Steel Construction) : poteaux tous les 7,5 m, trois places de 2,5 m entre poteaux.
15. Poutre plate précontrainte (Batiproduits) : retombée de 15 cm pour 8 à 10 m de portée.
16. ByBéton, « Qu'est-ce que le béton banché ? » : un voile fait en général 15 à 20 cm, jamais moins de 12. https://bybeton.fr/?p=22427
17. Neufert p. 274-276 : exemple de trame de 7,95 m pour trois places de 2,65 m ; les grandes portées réduisent de 7 à 12 % la surface prise par les poteaux.

**Tenue au feu**

18. Arrêté du 31 janvier 1986, article 81 (parcs des bâtiments d'habitation) : stabilité au feu d'une heure et demie au-delà de deux niveaux, jusqu'à 28 m. Notice de sécurité d'un parc de neuf niveaux à Cagnes-sur-Mer, article PS 6 : structures R 90, planchers REI 90.
19. Guide belge d'application de l'Eurocode 2, tableau repris de l'EN 1992-1-2 (tableau 5.9) : dalle sans poutre de 200 mm pour REI 90.
