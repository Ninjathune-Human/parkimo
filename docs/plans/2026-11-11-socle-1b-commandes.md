# Socle 1b : commandes, découpage de l'interface, intégration continue

**But.** Toute action sur l'aménagement peut passer par une commande JSON appliquée par le moteur : c'est ce qu'utiliseront l'IA intégrée et le serveur MCP (lot 3).

**Spécification.** `docs/2026-11-11-audit-et-direction.md`, § 5, décisions 1, 4 et 7.

## Décisions

- **Le projet garde le format de l'interface** (`site`, `params`). `src/projet.rs` le lit tel quel, conserve les champs qu'il ne connaît pas, et reproduit en Rust la conversion de la page vers l'entrée du moteur. La page reste propriétaire du format tant que ses gestes n'ont pas migré.
- **La référence de la conversion est l'interface.** `tests/fixtures/projets.json` est produit dans Chrome par `tools/essais/projets_reference.py` : cinq projets, l'entrée que la page en tire, le nombre de places. À régénérer quand `engineInput` change.
- **Une seule table des réglages** (`projet::REGLAGES`) : valeur par défaut de l'interface et bornes admises par les commandes. Les bornes sont larges (largeur de place 1,80 à 4,00 m, allée 2,50 à 12 m, pente 5 à 18 %) : elles écartent les valeurs absurdes, la conformité juge le reste.
- **Une seule table des commandes** (`commandes::OPS`) : exécution, description, paramètres, exemple. Le test du catalogue rejoue chaque exemple.
- **Transaction.** Une liste passe en entier ou pas du tout ; le refus donne le rang et le motif, en français.
- **Les gestes de l'interface ne passent pas encore par les commandes.** `appliquer(commandes)` existe dans la page (et `window.parkimo`), s'annule comme un geste, mais glisser un sommet modifie toujours le projet directement. Migration geste par geste, quand un geste est repris.
- **Interface en fichiers** : squelette, styles, quinze scripts par section, assemblés par `tools/page.py` ; page identique à l'octet près au moment du découpage.
- **Tests longs à part** : deux tests marqués `#[ignore]`, lancés par la tâche `tests-longs`.
- **Tag `deploye`** : flux `tag-deploye.yml`, déclenché par la fin du déploiement GitHub Pages.

## Reste ouvert

- Sortir les pages compilées du dépôt demande de régler GitHub Pages sur « GitHub Actions » (réglage du dépôt, hors de portée d'ici).
- Commandes absentes : voies du squelette (sens, nombre de voies), noues, noyaux du silo, trajectoires de giration, objets à sommets courbes.
- Lecture pour une IA : le plan complet est volumineux ; prévoir un résumé (bilan, constats, objets et leurs rangs).
