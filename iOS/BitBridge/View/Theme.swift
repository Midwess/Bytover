//
//  Theme.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import SwiftUI

enum Theme: String {
    case PrimaryViolet
    case SecondaryVividViolet
    case TertiaryViolet
    case DarkViolet
    case LightViolet
    case SecondaryViolet
    case Pink
    case SecondaryBlue
    
    static var gradientHeading: LinearGradient {
        get {
            LinearGradient(
                colors: [Theme.LightViolet.color, Theme.LightViolet.color.opacity(0.7)],
                startPoint: .top,
                endPoint: .bottom
            )
        }
    }
    
    static func starGradient(x: Double, y: Double) -> AngularGradient {
        AngularGradient(colors: [
            Theme.DarkViolet.color.opacity(0.1),
            Theme.Pink.color.opacity(0.1),
            Theme.PrimaryViolet.color.opacity(0.05),
            Theme.DarkViolet.color.opacity(0.08)
        ], center: .init(x: x, y: y), angle: .degrees(60))
    }
    
    static var darkBgGradient: LinearGradient {
        get {
            LinearGradient(colors: [
                Theme.SecondaryVividViolet.color,
                Theme.DarkViolet.color
            ], startPoint: .top, endPoint: .bottom)
        }
    }
    
    static var primaryGradient: LinearGradient {
        get {
            LinearGradient(
                colors: [Theme.PrimaryViolet.color, Theme.SecondaryVividViolet.color],
                startPoint: .top,
                endPoint: .bottom
            )
        }
    }
    
    var color: Color {
        get {
            Color(rawValue)
        }
    }
    
    var string: String {
        get {
            rawValue
        }
    }
}

enum ImageAsset: String {
    case SupaLighting
    
    var image: Image {
        get {
            Image(self.string)
        }
    }
    
    var string: String {
        get {
            rawValue
        }
    }
}

struct FontTheme {
    static var H1Heading: Font {
        get {
            .title
                .weight(.bold)
        }
    }
    
    static var H2Heading: Font {
        get {
            .title2.weight(.bold)
        }
    }
    
    static var Body1: Font {
        get {
            .body.weight(.bold)
        }
    }
    
    static var Body2: Font {
        get {
            .body
        }
    }
}

enum SpaceTheme {
    case screen
    
    var value: CGFloat {
        get {
            switch self {
                case .screen:
                    return 24
            }
        }
    }
}
