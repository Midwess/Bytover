//
//  ShareView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation

public struct ShareView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject var core: Core
    @State private var startTime = Date.now
    
    public var body: some View {
        ZStack(alignment: .bottom) {
            Theme.darkBgGradient.opacity(0.3)
            StunningBackgroundGradientAnimation()
            ScrollView {
                VStack {
                    LogoView(width: 65)
                        .frame(width: .infinity, height: 100)
                    Text("Secure and Fastest \n file transfer")
                        .multilineTextAlignment(.center)
                        .modifier(Label1())
                        .foregroundStyle(Theme.gradientHeading)
                        .padding(.top, 10)
                    UpgradePremiumButton()
                    
                    ForEach(core.transfer?.selected_resources ?? [], id: \.self) { item in
                        Text(item.name)
                    }
                    
                    ContentPickerView()
                }
            }
            
            ShareButton(width: 150)
                .padding(.bottom, 80)
        }
        .onAppear() {
            print("On appear")
            core.update(.transfer(.initSession))
        }
        .ignoresSafeArea()
    }
}

#Preview {
    ShareView()
}
