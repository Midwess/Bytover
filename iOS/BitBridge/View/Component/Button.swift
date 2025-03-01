//
//  Button.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct PrimaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .frame(maxWidth: .infinity)
            .padding()
            .font(FontTheme.H2Heading)
            .foregroundColor(Theme.PrimaryText.color)
            .background(Theme.BluePrimary.color)
            .cornerRadius(.infinity)
    }
}

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
            Text("Upgrade to premium")
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .modifier(Label3())
                .foregroundStyle(Theme.PrimaryText.color.opacity(1))
                .background {
                    RoundedRectangle(cornerRadius: .infinity)
                        .fill(Theme.SeaTertiary.color.opacity(0.1))
                }
        }
    }
}

struct AddFileButton: View {
    var body: some View {
        Button(action: {}) {
            HStack {
                Text("Add new files")
                    .padding(.trailing, 4)
                Image(systemName: "plus")
                    .resizable()
                    .padding(.all, 8)
                    .frame(width: 30, height: 30)
            }
        }
        .background(Theme.BluePrimary.color)
        .foregroundColor(Theme.PrimaryText.color)
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

struct ButtonNavigation: View {
    var icon: Image
    var icon_selected: Image
    var isSelected: Bool
    
    // Add a default value for isSelected to maintain backward compatibility
    init(icon: Image, icon_selected: Image, isSelected: Bool = false) {
        self.icon = icon
        self.icon_selected = icon_selected
        self.isSelected = isSelected
    }
    
    var body: some View {
        VStack {
            // Use the selected icon when isSelected is true
            (isSelected ? icon_selected : icon)
                .resizable()
                .frame(width: 20, height: 20)
        }
    }
}

struct ShareButton: View {
    let width: CGFloat
    @State private var startTime = Date.now
    var body: some View {
        ZStack {
            Group {
                ZStack {
                    TimelineView(.animation) { timeline in
                        let elapsedTime = startTime.distance(to: timeline.date)
                        Circle()
                            .fill(Theme.GreenSecondary.color.opacity(0.6))
                            .visualEffect { content, proxy in
                                content
                                    .colorEffect(
                                        ShaderLibrary.circleWave(
                                            .float2(proxy.size),
                                            .color(Theme.BluePrimary.color),
                                            .float(elapsedTime * 0.8)
                                        )
                                    )
                            }
                    }
                    Button(action: {}) {
                        ImageAsset.SendEmpty.image
                            .rotationEffect(.degrees(-45))
                            .opacity(0.8)
                            .offset(x: 1, y: -1)
                    }
                    .frame(width: width * 0.49, height: width * 0.49)
                    .background(Theme.BlackBase.color)
                    .clipShape(Circle())
                }
            }
        }
        .frame(width: width, height: width)
        .ignoresSafeArea()
    }
}

#Preview("Bottom navigation") {
    ButtonNavigation(icon: Image(systemName: "plus"), icon_selected: Image("plus.fill"), isSelected: false)
}

#Preview("Premium button") {
    UpgradePremiumButton()
        .previewLayout(.sizeThatFits)
        .padding()
        .background(Theme.DarkViolet.color)
}

#Preview("Add file button") {
    AddFileButton()
        .previewLayout(.sizeThatFits)
        .padding()
        .background(Theme.DarkViolet.color)
}

#Preview("Share button") {
    ShareButton(width: 300)
}
