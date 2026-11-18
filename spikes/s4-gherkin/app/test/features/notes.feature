# language: fr
Fonctionnalité: Gestion de notes
  Pour ne rien oublier
  En tant qu'utilisateur
  Je veux gérer une liste de notes

  Contexte:
    Soit une liste de notes vide

  @core @gui
  Scénario: Ajouter une note
    Quand j'ajoute la note "Acheter du pain" avec la priorité 3
    Alors la liste contient 1 note
    Et la note "Acheter du pain" a la priorité 3

  @core
  Scénario: Cœur seul, flottant et mot isolé
    Quand j'ajoute la note "Lire" avec la priorité 2
    Et j'applique un coefficient de 1.5
    Alors la priorité effective de "Lire" est 3
    Et le statut est ouvert

  @core @gui
  Plan du scénario: Ajouter plusieurs notes
    Quand j'ajoute la note "<titre>" avec la priorité <priorite>
    Alors la liste contient 1 note
    Et la note "<titre>" a la priorité <priorite>

    Exemples:
      | titre   | priorite |
      | Courses | 1        |
      | Sport   | 5        |

  @core @gui
  Scénario: Ajouter plusieurs notes d'un coup
    Quand j'ajoute les notes suivantes:
      | "titre"   | "priorite" |
      | "Courses" | 1        |
      | "Sport"   | 5        |
    Alors la liste contient 2 notes

  Règle: Une note vide est refusée

    @core @gui
    Scénario: Titre vide
      Quand j'ajoute la note "" avec la priorité 1
      Alors la liste contient 0 note
