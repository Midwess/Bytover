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
    case SecondaryDarkViolet
    case SecondaryBlue
    case LightPrimaryViolet
    case GreenSecondary
    case BluePrimary
    case PrimaryText
    case SecondaryText
    case SeaTertiary
    case BlueViolet
    case Orange
    case LightSea
    case DarkBlue
    case BlackBase
    case BlackSecondary
    case Navy
    case BlueSky
    case FileColor
    case DocumentColor
    case PrimaryBackground
    case Gray

    static var gradientHeading: LinearGradient {
        get {
            LinearGradient(
                colors: [Theme.BlackBase.color.opacity(0.0), Theme.BlackBase.color],
                startPoint: .top,
                endPoint: .bottom
            )
        }
    }

   static var gradientHeading2: LinearGradient {
        get {
            LinearGradient(
                colors: [Theme.SecondaryVividViolet.color.opacity(0.4), Theme.DarkViolet.color.opacity(0.3)],
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
                Theme.BlackBase.color.opacity(0.0),
                Theme.BlackBase.color.opacity(1)
            ], startPoint: .top, endPoint: .bottom)
        }
    }

    static var circlePrimaryGradient: RadialGradient {
        get {
            RadialGradient(colors: [
                Theme.DarkViolet.color,
                Theme.SecondaryVividViolet.color
            ], center: .center, startRadius: 0.0, endRadius: 80.0)
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

    static var textGradient: LinearGradient {
        get {
            LinearGradient(
                colors: [Theme.Pink.color, Theme.PrimaryViolet.color],
                startPoint: .leading,
                endPoint: .trailing
            )
        }
    }

    var color: Color {
        get {
            Color(rawValue)
        }
    }

    var uiColor: UIColor {
        get {
            UIColor(color)
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
    case GlobeEmpty
    case GlobeFull
    case SettingEmpty
    case SettingFull
    case PuzzelEmpty
    case PuzzelFull
    case SendEmpty
    case Folder
    case File
    case FilePdf
    case CameraVideo
    case CameraVideoSolid
    case FileImage
    case MailReceiveEmpty
    case MailReceiveFull
    case More
    case AndroidPhone
    case iPhone
    case Macbook
    case WindowLaptop
    case UserEmpty
    case DeviceEmpty
    case ArrowDown
    case Owl
    case Download
    case Upload
    case Edit
    case Link
    case Lock

    var image: Image {
        get {
            Image(self.string)
        }
    }

    var uiImage: UIImage {
        get {
            UIImage(named: self.string)!
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
            .largeTitle
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
            .title3.weight(.bold)
        }
    }

    static var Body2: Font {
        get {
            .body
        }
    }

    static var Label1: Font {
        get {
            .callout
        }
    }
}

enum SpaceTheme {
    case screen
    case item
    case cohesive

    var value: CGFloat {
        get {
            switch self {
            case .screen:
                return 24
            case .item:
                return 12
            case .cohesive:
                return 8
            }
        }
    }

    var halfValue: CGFloat {
        get {
            return value / 2
        }
    }

    var biggerValue: CGFloat {
        get {
            return value + Self.cohesive.value
        }
    }
}
