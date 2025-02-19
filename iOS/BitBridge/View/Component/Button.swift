//
//  Button.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct PrimaryGradientButton<S: ShapeStyle>: ButtonStyle {
    let gradient: S
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .frame(maxWidth: .infinity)
            .padding()
            .font(FontTheme.H2Heading)
            .foregroundColor(Theme.LightViolet.color)
            .background(gradient)
            .cornerRadius(.infinity)
            .overlay(
                RoundedRectangle(cornerRadius: .infinity)
                    .stroke(Theme.LightViolet.color.opacity(0.3), lineWidth: 1)
            )
            .shadow(color: Theme.LightViolet.color.opacity(0.2), radius: 15)
    }
}

struct PrimaryButton: View {
    var body: some View {
        Button(action: {}) {
            Text("Button")
        }
        .buttonStyle(PrimaryGradientButton(gradient: Theme.primaryGradient))
    }
}

struct UpgradePremiumButton: View {
    var body: some View {
        Button(action: {}) {
            Text("Upgrade premium")
                .padding(.horizontal, 8)
                .padding(.vertical, 2)
                .foregroundStyle(Theme.textGradient)
                .background {
                    RoundedRectangle(cornerRadius: .infinity)
                        .fill(Theme.LightViolet.color.opacity(0.1))
                        .stroke(Theme.PrimaryViolet.color.opacity(0.15), lineWidth: 1)
                }
        }
    }
}

struct Button_Preview: PreviewProvider {
    static var previews: some View {
        VStack {
            Spacer()
            PrimaryButton()
                .padding()
            Spacer()
        }
        .frame(width: .infinity, height: .infinity)
        .background(Theme.DarkViolet.color)
    }
}

#Preview("Premium button") {
    UpgradePremiumButton()
        .previewLayout(.sizeThatFits)
        .padding()
        .background(Theme.DarkViolet.color)
}
